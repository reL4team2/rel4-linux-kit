#![no_std]
#![no_main]
#![feature(never_type)]
#![feature(const_trait_impl)]

extern crate alloc;
extern crate sel4_panicking;

mod arch;
mod child_test;
mod device;
mod exception;
mod fs;
mod logging;
mod runtime;
// mod syscall;
mod task;
mod thread;
mod utils;

sel4_panicking_env::register_debug_put_char!(sel4::sys::seL4_DebugPutChar);

fn main() -> ! {
    // 初始化接收 IPC 传递的 Capability 的 Slot
    common::init_recv_slot();

    // 初始化 LOG
    logging::init();

    // 初始化 object allocator
    utils::obj::init();

    // 初始化文件系统
    fs::init();

    // 初始化设备
    device::init();

    // 添加测试子任务
    child_test::add_test_child().unwrap();

    // 循环处理异常(含伪 syscall)
    exception::waiting_and_handle();
}
