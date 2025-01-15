#![no_std]
#![no_main]

#[macro_use]
extern crate alloc;

use common::services::{block::BlockService, fs::FileServiceLabel, root::RootService};
use crate_consts::{DEFAULT_PARENT_EP, DEFAULT_SERVE_EP};
use cursor::DiskCursor;
use sel4::{cap::Endpoint, debug_print, debug_println, with_ipc_buffer_mut, MessageInfoBuilder};
use slot_manager::LeafSlot;

mod cursor;
mod runtime;

const ROOT_SERVICE: RootService = RootService::from_bits(DEFAULT_PARENT_EP);
const BLK_THREAD_EP_SLOT: Endpoint = Endpoint::from_bits(0x21);
const SERVE_EP: Endpoint = Endpoint::from_bits(DEFAULT_SERVE_EP);

fn main() -> ! {
    common::init_log!(log::LevelFilter::Trace);
    common::init_recv_slot();

    log::info!("Booting...");

    // FIXME: Using Common Consts instead of fixed constants
    let blk_ep = BlockService::new(BLK_THREAD_EP_SLOT);
    let blk_ep_slot = LeafSlot::new(0x21);

    ROOT_SERVICE
        .find_service("block-thread", blk_ep_slot)
        .expect("Can't find blk-thread service");

    blk_ep.ping().expect("Can't ping blk-thread service");

    let cursor: DiskCursor = DiskCursor::default();
    let fs = fatfs::FileSystem::new(cursor, fatfs::FsOptions::new()).expect("open fs wrong");

    let rev_msg = MessageInfoBuilder::default();
    loop {
        let (message, _) = SERVE_EP.recv(());
        let msg_label = match FileServiceLabel::try_from(message.label()) {
            Ok(label) => label,

            Err(_) => continue,
        };
        log::debug!("Recv <{:?}> len: {}", msg_label, message.length());
        match msg_label {
            FileServiceLabel::Ping => {
                with_ipc_buffer_mut(|ib| {
                    sel4::reply(ib, rev_msg.build());
                });
            }
            // FIXME: 应该返回一个结构，或者数组表示所有文件
            // 类似于 getdents
            FileServiceLabel::ReadDir => {
                log::debug!("Read Dir Message");

                fs.root_dir().iter().for_each(|f| {
                    debug_print!("{}\t", f.unwrap().file_name());
                });
                debug_println!();
                with_ipc_buffer_mut(|ipc_buffer| sel4::reply(ipc_buffer, rev_msg.build()));
            }
        }
    }
}
