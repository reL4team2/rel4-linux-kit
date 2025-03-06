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

mod child_test;
mod logging;

pub mod arch;
pub mod consts;
pub mod device;
pub mod exception;
pub mod fs;
pub mod syscall;
pub mod task;
pub mod utils;

sel4_runtime::entry_point!(main);

macro_rules! test_task {
    ($file:expr) => {{
        const CHILD_ELF: &[u8] = include_bytes_aligned::include_bytes_aligned!(
            16,
            concat!("../../../testcases/", $file)
        );
        child_test::add_test_child(CHILD_ELF, &[$file]).unwrap();
    }};
}

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

    // // TODO: Make elf file path dynamically available.
    // const CHILD_ELF: &[u8] =
    //     include_bytes_aligned::include_bytes_aligned!(16, "../../../testcases/test_echo");
    // const CHILD_ELF1: &[u8] =
    //     include_bytes_aligned::include_bytes_aligned!(16, "../../../testcases/unlink");

    // // 添加测试子任务
    // child_test::add_test_child(CHILD_ELF, &["busybox", "sh"]).unwrap();
    // child_test::add_test_child(CHILD_ELF1, &["busybox", "sh"]).unwrap();

    test_task!("brk");
    test_task!("chdir");
    test_task!("close");
    test_task!("dup");
    test_task!("dup2");
    test_task!("fstat");
    test_task!("getcwd");
    test_task!("getpid");
    test_task!("getppid");
    test_task!("gettimeofday");
    test_task!("mkdir_");
    test_task!("open");
    test_task!("openat");
    test_task!("read");
    test_task!("test_echo");
    test_task!("uname");
    test_task!("unlink");
    test_task!("write");

    // 循环处理异常(含伪 syscall)
    exception::waiting_and_handle();
}
