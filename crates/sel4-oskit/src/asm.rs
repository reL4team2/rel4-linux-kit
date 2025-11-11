//! 提供一些代替汇编实现的功能
//! 如果将传统操作系统移植到 seL4 上，一些直接的汇编操作无法执行，只能使用这些代替函数
use memory_addr::PhysAddr;

/// 写入内核页表地址
pub unsafe fn write_kernel_page_table(_root_paddr: PhysAddr) {}

/// 等待中断
pub fn wait_for_irqs() {
    sel4::r#yield();
}
