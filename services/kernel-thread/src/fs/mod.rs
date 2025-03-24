//! 初始化文件系统服务
//!
//! 查找 root-task 中存在的串口服务，并记录到全局变量中

pub mod file;
pub mod ipc_fs;
pub mod pipe;
pub mod stdio;
pub mod vfs;

use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use common::services::root::create_channel;
use ipc_fs::IPCFileSystem;
use spin::Mutex;
use vfs::{FileResult, FileSystem};

static MOUNTED_FS: Mutex<Vec<(String, Arc<dyn FileSystem>)>> = Mutex::new(Vec::new());

pub(super) fn init() {
    // 寻找 fs_service 并尝试 ping
    let mut ipc_fs = IPCFileSystem::new("fs-thread").expect("can't find service");
    ipc_fs.fs.ping().unwrap();
    let channel_id = create_channel(0x3_0000_0000, 4).unwrap();
    ipc_fs.fs.init(channel_id, 0x3_0000_0000, 0x4000).unwrap();
    MOUNTED_FS
        .lock()
        .push((String::from("/"), Arc::new(ipc_fs)));
}

/// 根据路径获取已经挂载的文件系统
///
/// 最大匹配原则，优先匹配最深的路径
/// FIXME: 使用 path 匹配不同的路径
#[inline]
pub fn get_mounted(path: &str) -> (String, Arc<dyn FileSystem>) {
    let ret = MOUNTED_FS
        .lock()
        .iter()
        .fold(None, |acc, x| {
            log::debug!("path: {}  self: {}", path, x.0);
            if !path.starts_with(&x.0) {
                return acc;
            }
            if x.0.len()
                > acc
                    .map(|y: &(String, Arc<dyn FileSystem>)| y.0.len())
                    .unwrap_or(0)
            {
                Some(x)
            } else {
                acc
            }
        })
        .cloned()
        .unwrap();
    log::debug!("get ret: {:#x?}", ret.0);
    ret
    // MOUNTED_FS.lock()[0].clone()
}

/// 挂载一个文件系统到指定的路径
///
/// - `path`  需要挂载到的路径
/// - `fs`    需要挂载的文件系统
#[inline]
pub fn mount(path: &str, fs: Arc<dyn FileSystem>) -> FileResult<()> {
    MOUNTED_FS.lock().push((path.to_string(), fs));
    Ok(())
}

/// 卸载一个已经挂载的文件系统
///
/// - `path`  需要卸载的文件系统的路径
#[inline]
pub fn umount(path: &str) -> FileResult<()> {
    MOUNTED_FS.lock().retain(|x| (x.0 != path));
    Ok(())
}
