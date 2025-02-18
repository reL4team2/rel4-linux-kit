use alloc::{string::String, vec::Vec};

/// 任务的信息
#[derive(Default)]
pub struct TaskInfo {
    /// 参数列表
    pub args: Vec<String>,
    /// 程序的入口地址
    pub entry: usize,
    /// vsyscall 段的地址
    pub vsyscall: usize,
    /// shim 地址
    pub shim_addr: usize,
}
