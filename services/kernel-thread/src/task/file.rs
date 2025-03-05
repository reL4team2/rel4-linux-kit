use alloc::string::String;

pub struct TaskFileInfo {
    /// 工作目录
    pub work_dir: String,
}

impl Default for TaskFileInfo {
    fn default() -> Self {
        Self {
            work_dir: String::from("/"),
        }
    }
}
