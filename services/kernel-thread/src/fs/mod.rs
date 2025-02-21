//! 初始化文件系统服务
//!
//! 查找 root-task 中存在的串口服务，并记录到全局变量中
use common::services::{fs::FileSerivce, root::find_service};
use spin::Once;

use crate::utils::obj::alloc_slot;

static FS_SERVICE: Once<FileSerivce> = Once::new();

pub(super) fn init() {
    // 寻找 fs_service 并尝试 ping
    FS_SERVICE.call_once(|| {
        let slot = alloc_slot();
        find_service("fs-thread", slot).expect("can't find service");
        slot.into()
    });
}
