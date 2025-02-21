use common::services::{fs::FileSerivce, root::find_service};
use spin::Once;

use crate::utils::obj::alloc_slot;

static FS_SERVICE: Once<FileSerivce> = Once::new();

pub fn init() {
    // 寻找 fs_service 并尝试 ping
    FS_SERVICE.call_once(|| {
        let slot = alloc_slot();
        find_service("fs-thread", slot).expect("can't find service");
        slot.into()
    });
}
