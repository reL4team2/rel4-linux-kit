use alloc::{string::String, sync::Arc};
use flatten_objects::FlattenObjects;
use spin::Mutex;
use syscalls::Errno;

use crate::{consts::fd::FD_CUR_DIR, fs::file::File};

use super::Sel4Task;

#[derive(Clone)]
pub struct TaskFileInfo {
    /// 工作目录
    pub work_dir: String,
    /// 文件描述符
    pub file_ds: Arc<Mutex<FlattenObjects<Arc<Mutex<File>>, 0x200>>>,
}

impl Default for TaskFileInfo {
    fn default() -> Self {
        let file_ds = Arc::new(Mutex::new(FlattenObjects::new()));
        Self {
            work_dir: String::from("/"),
            file_ds,
        }
    }
}

impl Sel4Task {
    /// 处理地址
    ///
    /// - `dir_fd` 需要读取的父文件夹
    /// - `path`   需要打开的文件地址
    pub fn deal_path(&self, dir_fd: isize, path: *const u8) -> Result<String, Errno> {
        let path_bytes = self.read_cstr(path as _).unwrap();
        let path = String::from_utf8(path_bytes).unwrap();

        let dir_path = if dir_fd == FD_CUR_DIR {
            self.file.work_dir.clone()
        } else if dir_fd > 0 {
            let dir = self
                .file
                .file_ds
                .lock()
                .get(dir_fd as _)
                .ok_or(Errno::EBADF)?
                .clone();

            dir.lock().path() + "/"
        } else {
            panic!("not supported")
        };

        if let Some(strip_path) = path.strip_prefix("./") {
            Ok(dir_path + strip_path)
        } else if path.starts_with("..") {
            panic!("not supported")
        } else if path == "." {
            Ok(dir_path)
        } else if path.starts_with("/") {
            Ok(path)
        } else {
            Ok(dir_path + &path)
        }
    }
}
