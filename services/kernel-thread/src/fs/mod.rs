//! 初始化文件系统服务
//!
//! 查找 root-task 中存在的串口服务，并记录到全局变量中

pub mod file;
pub mod ipc_fs;
pub mod pipe;
pub mod stdio;
pub mod vfs;

use alloc::{string::String, sync::Arc, vec::Vec};
use ipc_fs::IPCFileSystem;
use spin::Mutex;
use vfs::FileSystem;

static MOUNTED_FS: Mutex<Vec<(String, Arc<dyn FileSystem>)>> = Mutex::new(Vec::new());

pub(super) fn init() {
    // // 寻找 fs_service 并尝试 ping
    // FS_SERVICE.call_once(|| {
    //     find_service("fs-thread")
    //         .expect("can't find service")
    //         .into()
    // });
    // FS_SERVICE.get().unwrap().ping().unwrap();
    let ipc_fs = IPCFileSystem::new("fs-thread").expect("can't find service");
    ipc_fs.fs.ping().unwrap();
    MOUNTED_FS
        .lock()
        .push((String::from("/"), Arc::new(ipc_fs)));
}

/// 根据路径获取已经挂载的文件系统
///
/// 最大匹配原则，优先匹配最深的路径
/// FIXME: 使用 path 匹配不同的路径
#[inline]
fn get_mounted(_path: &str) -> (String, Arc<dyn FileSystem>) {
    MOUNTED_FS.lock()[0].clone()
}
