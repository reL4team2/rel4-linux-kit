#![no_std]
#![no_main]

#[macro_use]
extern crate alloc;

use common::services::{block::BlockService, fs::FileEvent, root::find_service};
use crate_consts::DEFAULT_SERVE_EP;
use cursor::DiskCursor;
use sel4::{debug_print, debug_println, with_ipc_buffer_mut, MessageInfoBuilder};

mod cursor;

sel4_runtime::entry_point!(main);

fn main() -> ! {
    common::init_log!(log::LevelFilter::Trace);
    common::init_recv_slot();

    log::info!("Booting...");

    // FIXME: Using Common Consts instead of fixed constants

    let recv_slot = find_service("block-thread").expect("Can't find blk-thread service");
    let blk_ep = BlockService::from(recv_slot);

    blk_ep.ping().expect("Can't ping blk-thread service");

    let cursor: DiskCursor = DiskCursor::new(blk_ep);
    let fs = fatfs::FileSystem::new(cursor, fatfs::FsOptions::new()).expect("open fs wrong");

    let rev_msg = MessageInfoBuilder::default();
    loop {
        let (message, _) = DEFAULT_SERVE_EP.recv(());
        let msg_label = match FileEvent::try_from(message.label()) {
            Ok(label) => label,

            Err(_) => continue,
        };
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

                fs.root_dir().iter().for_each(|f| {
                    debug_print!("{}\t", f.unwrap().file_name());
                });
                debug_println!();
                with_ipc_buffer_mut(|ipc_buffer| sel4::reply(ipc_buffer, rev_msg.build()));
            }
            FileEvent::Unknown(label) => {
                log::warn!("Unknown label: {}", label);
            }
            others => log::warn!("not inplemented {:?}", others),
        }
    }
}
