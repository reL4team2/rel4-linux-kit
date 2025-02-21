//! 设备相关的模块
//!
//! 初始化设备相关的模块，通过 ipc 向 root-task 查找特定的服务

pub mod uart;

pub(super) fn init() {
    uart::init();
}
