use alloc::{string::String, vec::Vec};

/// 任务的信息
#[derive(Default, Clone)]
pub struct TaskInfo {
    /// 参数列表
    pub args: Vec<String>,
    /// 程序的入口地址
    pub entry: usize,
    /// 程序的结尾位置
    pub task_vm_end: usize,
}
