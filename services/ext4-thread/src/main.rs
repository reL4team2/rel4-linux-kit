#![no_std]
#![no_main]

#[macro_use]
extern crate alloc;

mod imp;

use alloc::{sync::Arc, vec::Vec};
use common::{
    consts::{IPC_DATA_LEN, REG_LEN},
    services::{block::BlockService, fs::FileEvent, root::find_service, IpcBufferRW},
};
use crate_consts::DEFAULT_SERVE_EP;
use ext4_rs::{Ext4, Ext4DirEntry, Ext4File};
use hashbrown::HashMap;
use imp::Ext4Disk;
use sel4::{debug_print, debug_println, with_ipc_buffer, with_ipc_buffer_mut, MessageInfoBuilder};

sel4_runtime::entry_point!(main);

const BLK_SERVICE: BlockService = BlockService::from_bits(0x21);

fn main() -> ! {
    common::init_log!(log::LevelFilter::Debug);
    common::init_recv_slot();

    log::info!("Booting...");

    let recv_slot = find_service("block-thread").unwrap();
    recv_slot.move_to(BLK_SERVICE.into()).unwrap();

    BLK_SERVICE.ping().unwrap();
    log::info!("Found Block Thread, It reply ping message");

    // 创建 Ext4 文件系统
    let disk = Arc::new(Ext4Disk);
    let ext4 = Ext4::open(disk);

    let mut open_files: HashMap<u32, Ext4File> = HashMap::new();

    let rev_msg = MessageInfoBuilder::default();
    loop {
        let (message, _) = DEFAULT_SERVE_EP.recv(());
        let msg_label = FileEvent::from(message.label());
        log::debug!("Recv <{:?}> len: {}", msg_label, message.length());
        match msg_label {
            FileEvent::Ping => {
                with_ipc_buffer_mut(|ib| {
                    sel4::reply(ib, rev_msg.build());
                });
            }
            // FIXME: 应该返回一个结构，或者数组表示所有文件
            // 类似于 getdents
            FileEvent::ReadDir => {
                log::debug!("Read Dir Message");
                let dir_entry: Vec<Ext4DirEntry> = ext4.read_dir_entry(2);
                for entry in dir_entry {
                    debug_print!("{}\t", entry.get_name());
                }
                debug_println!();
                with_ipc_buffer_mut(|ipc_buffer| sel4::reply(ipc_buffer, rev_msg.build()));
            }
            FileEvent::Open => {
                with_ipc_buffer_mut(|ib| {
                    let mut offset = 0;
                    let path = <&str>::read_buffer(ib, &mut offset);
                    let mut ext_file = Ext4File::new();
                    ext4.ext4_open(&mut ext_file, &path, "w+", true).unwrap();
                    with_ipc_buffer_mut(|ib| {
                        ib.msg_regs_mut()[0] = ext_file.inode as _;
                        ib.msg_regs_mut()[1] = ext_file.fsize;

                        sel4::reply(ib, rev_msg.length(2).build());
                    });
                    open_files.insert(ext_file.inode, ext_file);
                });
            }
            FileEvent::ReadAt => {
                let (inode, offset) =
                    with_ipc_buffer(|ib| (ib.msg_regs()[0] as u32, ib.msg_regs()[1] as _));
                if let Some(ext4_file) = open_files.get_mut(&inode) {
                    ext4_file.fpos = offset;
                    let mut buffer = vec![0u8; IPC_DATA_LEN - REG_LEN];
                    let mut rlen = 0;
                    let size = buffer.len();
                    ext4.ext4_file_read(ext4_file, &mut buffer, size, &mut rlen)
                        .unwrap();
                    with_ipc_buffer_mut(|ib| {
                        ib.msg_regs_mut()[0] = rlen as _;
                        ib.msg_bytes_mut()[REG_LEN..REG_LEN + rlen]
                            .copy_from_slice(&buffer[..rlen]);
                        sel4::reply(ib, rev_msg.length(1 + rlen.div_ceil(REG_LEN)).build());
                    })
                }
            }
            FileEvent::Unknown(label) => {
                log::warn!("Unknown label: {}", label);
            }
        }
    }
}
