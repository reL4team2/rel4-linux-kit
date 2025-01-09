#![no_std]
#![no_main]

#[macro_use]
extern crate alloc;

use common::services::{block::BlockService, root::RootService};
use crate_consts::DEFAULT_PARENT_EP;
use cursor::DiskCursor;
use sel4::cap::Endpoint;
use slot_manager::LeafSlot;

mod cursor;
mod runtime;

static ROOT_SERVICE: RootService = RootService::from_bits(DEFAULT_PARENT_EP);
const BLK_THREAD_EP_SLOT: Endpoint = Endpoint::from_bits(0x21);

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
    let inner = fatfs::FileSystem::new(cursor, fatfs::FsOptions::new()).expect("open fs wrong");

    inner.root_dir().iter().for_each(|f| {
        log::debug!("dir entry: {:?}", f);
    });
    log::debug!("display entry end");
    loop {}
}
