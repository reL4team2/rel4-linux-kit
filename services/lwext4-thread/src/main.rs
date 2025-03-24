#![no_std]
#![no_main]

extern crate alloc;

mod imp;

use core::{iter::zip, mem::ManuallyDrop};

use alloc::string::String;
use common::{
    consts::{DEFAULT_SERVE_EP, IPC_DATA_LEN, REG_LEN},
    services::{
        IpcBufferRW,
        block::BlockService,
        fs::{Dirent64, FileEvent, Stat, StatMode},
        root::{create_channel, find_service, join_channel},
    },
};
use flatten_objects::FlattenObjects;
use imp::Ext4Disk;
use lwext4_rust::{
    Ext4BlockWrapper, Ext4File, InodeTypes,
    bindings::{O_CREAT, O_TRUNC},
};
use sel4::{IpcBuffer, MessageInfoBuilder, with_ipc_buffer_mut};
use sel4_runtime::utils::alloc_free_addr;
use spin::Lazy;
use syscalls::Errno;

sel4_runtime::entry_point!(main);

const O_DIRECTORY: u32 = 0o40000;
const STORE_CAP: usize = 500;
static BLK_SERVICE: Lazy<BlockService> = Lazy::new(|| find_service("block-thread").unwrap().into());

fn main() -> ! {
    common::init_log!(log::LevelFilter::Error);
    common::init_recv_slot();

    log::info!("Booting...");

    BLK_SERVICE.ping().unwrap();
    let channel_id = create_channel(0x3_0000_0000, 4).unwrap();
    BLK_SERVICE.init(channel_id).unwrap();

    let mut stores = FlattenObjects::<Ext4File, STORE_CAP>::new();
    let mut mapped = FlattenObjects::<(usize, usize), 32>::new();
    let _ = ManuallyDrop::new(Ext4BlockWrapper::<Ext4Disk>::new(Ext4Disk::new()).unwrap());

    loop {
        with_ipc_buffer_mut(|ib| handle_events(&mut stores, &mut mapped, ib));
    }
}

fn handle_events(
    stores: &mut FlattenObjects<Ext4File, STORE_CAP>,
    mapped: &mut FlattenObjects<(usize, usize), 32>,
    ib: &mut IpcBuffer,
) {
    let (message, badge) = DEFAULT_SERVE_EP.recv(());
    let rev_msg = MessageInfoBuilder::default();

    let msg_label = FileEvent::from(message.label());
    log::debug!("Recv <{:?}> len: {}", msg_label, message.length());
    match msg_label {
        FileEvent::Ping => sel4::reply(ib, rev_msg.build()),
        FileEvent::Init => {
            let ptr = alloc_free_addr(0) as *mut u8;
            assert_eq!(message.length(), 1);
            let channel_id = with_ipc_buffer_mut(|ib| ib.msg_regs()[0] as _);
            let size = join_channel(channel_id, ptr as usize).unwrap();
            mapped
                .add_at(badge as _, (ptr as usize, channel_id))
                .map_err(|_| ())
                .unwrap();
            sel4::reply(ib, rev_msg.build());
            alloc_free_addr(size);
        }
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
            let addr = mapped.get(badge as usize).unwrap().0;
            let (inode, offset) = (ib.msg_regs()[0] as usize, ib.msg_regs()[1] as _);
            let buf_len = ib.msg_regs()[2] as usize;
            if let Some(ext4_file) = stores.get_mut(inode) {
                // let mut buffer = [0u8; IPC_DATA_LEN - REG_LEN];
                ext4_file.file_seek(offset, 0).unwrap();
                let buffer = unsafe { core::slice::from_raw_parts_mut(addr as _, buf_len) };
                let rlen = ext4_file.file_read(buffer).unwrap();

                ib.msg_regs_mut()[0] = rlen as _;
                sel4::reply(ib, rev_msg.length(1).build());
            } else {
                panic!("Can't Find File")
            }
        }
        FileEvent::WriteAt => {
            let (inode, offset) = (ib.msg_regs()[0] as usize, ib.msg_regs()[1] as _);
            let data_len = ib.msg_regs()[2] as usize;
            if let Some(ext4_file) = stores.get_mut(inode) {
                ext4_file.file_seek(offset, 0).unwrap();
                let data = ib.msg_bytes()[3 * REG_LEN..3 * REG_LEN + data_len].to_vec();
                let wlen = ext4_file.file_write(&data).unwrap();
                ib.msg_regs_mut()[0] = wlen as _;
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
        FileEvent::GetDents64 => {
            let inode = ib.msg_regs()[0] as usize;
            let mut offset = ib.msg_regs()[1] as usize;
            let rlen = (ib.msg_regs()[2] as usize).min(IPC_DATA_LEN - 2 * REG_LEN);
            if let Some(ext4_file) = stores.get_mut(inode) {
                let entries = ext4_file.lwext4_dir_entries().unwrap();
                let mut real_rlen: usize = 0;
                let mut base_ptr = ib.msg_bytes_mut().as_mut_ptr() as usize + 2 * REG_LEN;
                for (name, ty) in zip(entries.0, entries.1).skip(offset) {
                    log::debug!("{:?} , {:?}", String::from_utf8(name.clone()), ty);
                    let len = name.len() + size_of::<Dirent64>();
                    let aligned = (len + 7) / 8 * 8;
                    if real_rlen + aligned > rlen {
                        break;
                    }
                    let dirent = unsafe { (base_ptr as *mut Dirent64).as_mut() }.unwrap();
                    dirent.ftype = 0;
                    dirent.reclen = aligned as _;
                    dirent.ino = 0;
                    dirent.off = (real_rlen + aligned) as _;
                    unsafe {
                        dirent
                            .name
                            .as_mut_ptr()
                            .copy_from(name.as_ptr(), name.len());
                    }
                    real_rlen += aligned;
                    base_ptr += aligned;
                    offset += 1;
                }
                ib.msg_regs_mut()[0] = real_rlen as _;
                ib.msg_regs_mut()[1] = offset as _;
                sel4::reply(ib, rev_msg.length(2 + real_rlen.div_ceil(REG_LEN)).build());
            } else {
                panic!("Can't find folder")
            }
        }
        _ => {
            log::warn!("Unknown label: {:?}", msg_label);
        }
    }
}
