#![no_std]
#![no_main]

extern crate alloc;

mod imp;

use core::mem::ManuallyDrop;

use common::{
    consts::{IPC_DATA_LEN, REG_LEN},
    services::{
        IpcBufferRW,
        block::BlockService,
        fs::{FileEvent, Stat, StatMode},
        root::find_service,
    },
};
use crate_consts::DEFAULT_SERVE_EP;
use flatten_objects::FlattenObjects;
use imp::Ext4Disk;
use lwext4_rust::{
    Ext4BlockWrapper, Ext4File, InodeTypes,
    bindings::{O_CREAT, O_TRUNC},
};
use sel4::{IpcBuffer, MessageInfo, MessageInfoBuilder, with_ipc_buffer_mut};
use spin::Lazy;
use syscalls::Errno;

sel4_runtime::entry_point!(main);

const O_DIRECTORY: u32 = 0o40000;
const STORE_CAP: usize = 500;
static BLK_SERVICE: Lazy<BlockService> = Lazy::new(|| find_service("block-thread").unwrap().into());

fn main() -> ! {
    common::init_log!(log::LevelFilter::Warn);
    common::init_recv_slot();

    log::info!("Booting...");

    BLK_SERVICE.ping().unwrap();

    let mut stores = FlattenObjects::<Ext4File, STORE_CAP>::new();
    let _ = ManuallyDrop::new(Ext4BlockWrapper::<Ext4Disk>::new(Ext4Disk::new()).unwrap());

    loop {
        let (message, _) = DEFAULT_SERVE_EP.recv(());
        with_ipc_buffer_mut(|ib| handle_events(message, &mut stores, ib));
    }
}

fn handle_events(
    message: MessageInfo,
    stores: &mut FlattenObjects<Ext4File, STORE_CAP>,
    ib: &mut IpcBuffer,
) {
    let rev_msg = MessageInfoBuilder::default();

    let msg_label = FileEvent::from(message.label());
    log::debug!("Recv <{:?}> len: {}", msg_label, message.length());
    match msg_label {
        FileEvent::Ping => sel4::reply(ib, rev_msg.build()),
        FileEvent::Open => {
            // TODO: Open Directory
            let mut offset = 0;
            let flags = u32::read_buffer(ib, &mut offset);
            let path = <&str>::read_buffer(ib, &mut offset);

            let mut ext4_file = Ext4File::new("/", lwext4_rust::InodeTypes::EXT4_DE_DIR);
            if flags & O_CREAT == O_CREAT {
                if flags & O_DIRECTORY != O_DIRECTORY {
                    ext4_file = Ext4File::new(&path, lwext4_rust::InodeTypes::EXT4_DE_REG_FILE);
                    // FIXME: clean this O_TRUNC
                    ext4_file.file_open(&path, flags | O_TRUNC).unwrap();
                } else {
                    panic!("Just support create regular file");
                }
            } else if ext4_file.check_inode_exist(&path, InodeTypes::EXT4_DE_DIR) {
                ext4_file = Ext4File::new(&path, lwext4_rust::InodeTypes::EXT4_DE_DIR);
            } else if ext4_file.check_inode_exist(&path, InodeTypes::EXT4_DE_REG_FILE) {
                ext4_file = Ext4File::new(&path, lwext4_rust::InodeTypes::EXT4_DE_REG_FILE);
                ext4_file.file_open(&path, flags).unwrap();
            } else {
                sel4::reply(ib, rev_msg.label(Errno::EACCES.into_raw() as _).build());
                return;
            }

            let file_size = ext4_file.file_size();
            if let Ok(index) = stores.add(ext4_file) {
                ib.msg_regs_mut()[0] = index as _;
                ib.msg_regs_mut()[1] = file_size;

                sel4::reply(ib, rev_msg.length(2).build());
            } else {
                panic!("Can't add files");
            }
        }
        FileEvent::ReadAt => {
            let (inode, offset) = (ib.msg_regs()[0] as usize, ib.msg_regs()[1] as _);
            if let Some(ext4_file) = stores.get_mut(inode) {
                let mut buffer = [0u8; IPC_DATA_LEN - REG_LEN];
                ext4_file.file_seek(offset, 0).unwrap();
                let rlen = ext4_file.file_read(&mut buffer).unwrap();

                ib.msg_regs_mut()[0] = rlen as _;
                ib.msg_bytes_mut()[REG_LEN..REG_LEN + rlen].copy_from_slice(&buffer[..rlen]);
                sel4::reply(ib, rev_msg.length(1 + rlen.div_ceil(REG_LEN)).build());
            } else {
                panic!("Can't Find File")
            }
        }
        FileEvent::WriteAt => {
            let (inode, offset) = (ib.msg_regs()[0] as usize, ib.msg_regs()[1] as _);
            let data_len = ib.msg_regs()[2] as usize;
            if let Some(ext4_file) = stores.get_mut(inode) {
                ext4_file.file_seek(offset, 0).unwrap();
                let rlen = ext4_file
                    .file_write(&ib.msg_bytes()[3 * REG_LEN..3 * REG_LEN + data_len])
                    .unwrap();

                ib.msg_regs_mut()[0] = rlen as _;
                sel4::reply(ib, rev_msg.length(1).build());
            } else {
                panic!("Can't Find File")
            }
        }
        FileEvent::Mkdir => {
            let path = <&str>::read_buffer(ib, &mut 0);
            let mut ext4_file = Ext4File::new(&path, lwext4_rust::InodeTypes::EXT4_DE_DIR);
            ext4_file.dir_mk(&path).unwrap();
            sel4::reply(ib, rev_msg.build());
        }
        FileEvent::Unlink => {
            let path = <&str>::read_buffer(ib, &mut 0);
            let mut ext4_file = Ext4File::new(&path, lwext4_rust::InodeTypes::EXT4_DE_DIR);
            ext4_file.file_remove(&path).unwrap();
            sel4::reply(ib, rev_msg.build());
        }
        FileEvent::Close => {
            let index = ib.msg_regs()[0] as usize;
            if let Some(mut ext4_file) = stores.remove(index) {
                ext4_file.file_close().unwrap();
            }
            sel4::reply(ib, rev_msg.build());
        }
        FileEvent::Stat => {
            let inode = ib.msg_regs()[0] as usize;
            if let Some(ext4_file) = stores.get_mut(inode) {
                let mode = ext4_file.file_mode_get().unwrap()
                    | match ext4_file.get_type() {
                        InodeTypes::EXT4_DE_REG_FILE => StatMode::FILE,
                        InodeTypes::EXT4_DE_DIR => StatMode::DIR,
                        InodeTypes::EXT4_DE_CHRDEV => StatMode::CHAR,
                        InodeTypes::EXT4_DE_BLKDEV => StatMode::BLOCK,
                        InodeTypes::EXT4_DE_FIFO => StatMode::FIFO,
                        InodeTypes::EXT4_DE_SOCK => StatMode::SOCKET,
                        InodeTypes::EXT4_DE_SYMLINK => StatMode::LINK,
                        _ => StatMode::FILE,
                    }
                    .bits();
                let stat = Stat {
                    blksize: 0x200,
                    ino: inode as _,
                    mode,
                    nlink: 1,
                    size: ext4_file.file_size(),
                    ..Default::default()
                };
                let len = size_of::<Stat>() / REG_LEN;
                unsafe {
                    (ib.msg_bytes_mut().as_ptr() as *mut Stat).copy_from(&stat, 1);
                }
                sel4::reply(ib, rev_msg.length(len).build());
            } else {
                panic!("Can't Find File")
            }
        }
        _ => {
            log::warn!("Unknown label: {:?}", msg_label);
        }
    }
}
