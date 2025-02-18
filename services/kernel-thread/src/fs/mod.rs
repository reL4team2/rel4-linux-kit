use common::services::fs::FileSerivce;
use spin::Once;

use crate::utils::service::find_service;

static FS_SERVICE: Once<FileSerivce> = Once::new();

pub fn init() {
    // 寻找 fs_service 并尝试 ping
    FS_SERVICE.call_once(|| {
        find_service("fs-thread")
            .expect("can't find service")
            .into()
    });
}
