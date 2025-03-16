#![no_std]
#![no_main]

#[macro_use]
extern crate alloc;

mod imp;

use alloc::sync::Arc;
use common::{
    consts::{DEFAULT_SERVE_EP, IPC_DATA_LEN, REG_LEN},
    services::{IpcBufferRW, block::BlockService, fs::FileEvent, root::find_service, sel4_reply},
};
use ext4_rs::Ext4;
use imp::Ext4Disk;
use sel4::{MessageInfoBuilder, with_ipc_buffer, with_ipc_buffer_mut};

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
                todo!()
                // log::debug!("Read Dir Message");
                // let dir_entry: Vec<Ext4DirEntry> = ext4.read_dir_entry(2);
                // for entry in dir_entry {
                //     debug_print!("{}\t", entry.get_name());
                // }
                // debug_println!();
                // with_ipc_buffer_mut(|ipc_buffer| sel4::reply(ipc_buffer, rev_msg.build()));
            }
            FileEvent::Open => {
                with_ipc_buffer_mut(|ib| {
                    // TODO: Open Directory
                    let mut offset = 0;
                    let path = <&str>::read_buffer(ib, &mut offset);
                    let inode = ext4.ext4_file_open(&path, "r").unwrap();
                    let inode_ref = ext4.get_inode_ref(inode);
                    with_ipc_buffer_mut(|ib| {
                        ib.msg_regs_mut()[0] = inode as _;
                        ib.msg_regs_mut()[1] = inode_ref.inode.size();

                        sel4::reply(ib, rev_msg.length(2).build());
                    });
                });
            }
            FileEvent::ReadAt => {
                let (inode, offset) =
                    with_ipc_buffer(|ib| (ib.msg_regs()[0] as u32, ib.msg_regs()[1] as _));
                let inode_ref = ext4.get_inode_ref(inode);
                if inode_ref.inode_num == inode {
                    let mut buffer = vec![0u8; IPC_DATA_LEN - REG_LEN];
                    let rlen = ext4.read_at(inode, offset, &mut buffer).unwrap();
                    with_ipc_buffer_mut(|ib| {
                        ib.msg_regs_mut()[0] = rlen as _;
                        ib.msg_bytes_mut()[REG_LEN..REG_LEN + rlen]
                            .copy_from_slice(&buffer[..rlen]);
                        sel4::reply(ib, rev_msg.length(1 + rlen.div_ceil(REG_LEN)).build());
                    })
                }
            }
            FileEvent::Mkdir => {
                let path = with_ipc_buffer(|ib| <&str>::read_buffer(ib, &mut 0));
                log::debug!("mkdir: {}", "test_chdir");
                ext4.ext4_dir_mk(&path).unwrap();
                log::debug!("mdkir done");
                sel4_reply(rev_msg.build());
            }
            _ => {
                log::warn!("Unknown label: {:?}", msg_label);
            }
        }
    }
}
