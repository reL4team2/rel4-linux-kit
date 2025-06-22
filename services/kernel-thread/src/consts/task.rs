//! 任务相关设置和参数
//!
//!

/// 默认堆地址
pub const DEF_HEAP_ADDR: usize = 0x7000_0000;

/// 默认栈顶地址
pub const DEF_STACK_TOP: usize = 0x2_0000_0000;

/// 默认栈大小
pub const DEF_STACK_SIZE: usize = 0x1_0000;

/// 默认栈底地质
pub const DEF_STACK_BOTTOM: usize = 0x1_F000_0000;

/// 用户空间起始地址
pub const USPACE_BASE: usize = 0x1000;

/// 默认工作目录
pub const DEF_WORK_DIR: &str = "/";

/// 复制物理页的时候使用的地址
pub const PAGE_COPY_TEMP: usize = 0x8_0000_0000;

/// VDSO 内核线程加载的地址区域
pub const VDSO_REGION_KADDR: usize = 0x4_0000_0000;

/// VDSO KADDR
pub const VDSO_KADDR: usize = 0x4_0000_0000;

/// VDSO 用户程序加载的地址区域
pub const VDSO_REGION_APP_ADDR: usize = 0x4_0000_0000;

/// VDSO 用户程序加载的地址
pub const VDSO_APP_ADDR: usize = 0x4_0000_0000;

/// 默认的 VDSO 区域大小
pub const VDSO_AREA_SIZE: usize = 0x2000;
