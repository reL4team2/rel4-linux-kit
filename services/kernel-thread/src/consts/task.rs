//! 任务相关设置和参数
//!
//!

/// 默认堆地址
pub const DEF_HEAP_ADDR: usize = 0x7000_0000;

/// 默认栈顶地址
pub const DEF_STACK_TOP: usize = 0x2_0000_0000;

/// 默认栈大小
pub const DEF_STACK_SIZE: usize = 0x1_0000;

/// 用户空间起始地址
pub const USPACE_BASE: usize = 0x1000;

/// 默认工作目录
pub const DEF_WORK_DIR: &str = "/";

/// 复制物理页的时候使用的地址
pub const PAGE_COPY_TEMP: usize = 0x8_0000_0000;
