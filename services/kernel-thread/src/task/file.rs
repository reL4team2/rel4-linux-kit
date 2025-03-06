use alloc::{string::String, sync::Arc};
use flatten_objects::FlattenObjects;
use spin::Mutex;

use crate::fs::file::File;

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
