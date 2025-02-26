//! 宏内核线程服务，这个线程可以将传统宏内核程序作为子程序运行，可以为子程序提供文件系统、设备等服务。
//! 目前还需要对需要运行的子程序进行预处理。
#![no_std]
#![no_main]
#![deny(missing_docs)]
#![feature(never_type)]
#![feature(const_trait_impl)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate alloc;
extern crate sel4_panicking;

mod child_test;
mod logging;
mod runtime;

pub mod arch;
pub mod consts;
pub mod device;
pub mod exception;
pub mod fs;
pub mod syscall;
pub mod task;
pub mod utils;

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
