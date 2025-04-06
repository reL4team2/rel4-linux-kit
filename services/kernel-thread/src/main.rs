//! 宏内核线程服务，这个线程可以将传统宏内核程序作为子程序运行，可以为子程序提供文件系统、设备等服务。
//! 目前还需要对需要运行的子程序进行预处理。
#![no_std]
#![no_main]
#![deny(missing_docs)]
#![deny(warnings)]
#![feature(never_type)]
#![feature(const_trait_impl)]

use futures::task::LocalSpawnExt;

#[macro_use]
extern crate log;
#[macro_use]
extern crate alloc;

#[macro_use]
pub mod rasync;

mod child_test;
mod logging;

pub mod arch;
pub mod consts;
pub mod device;
pub mod exception;
pub mod fs;
pub mod syscall;
pub mod task;
pub mod timer;
pub mod utils;

sel4_runtime::entry_point!(main);

// macro_rules! test_task {
//     ($file:expr $(,$args:expr)*) => {{
//         const CHILD_ELF: &[u8] = include_bytes_aligned::include_bytes_aligned!(
//             16,
//             concat!("../../../testcases/", $file)
//         );
//         child_test::add_test_child(CHILD_ELF, &[$file $(,$args)*]).unwrap();
//     }};
// }

macro_rules! test_task {
($file:expr $(,$args:expr)*) => {{
        let mut file =
            fs::file::File::open(concat!("/", $file), consts::fd::DEF_OPEN_FLAGS).unwrap();
        child_test::add_test_child(&file.read_all().unwrap(), &[$file $(,$args)*]).unwrap();
        sel4::debug_println!("loading file: {}", $file);
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

    // 初始化异常处理 Mixed IPC/Notification
    exception::init();

    // 初始化定时器
    timer::init();

    test_task!("busybox", "sh", "/init.sh");

    let mut pool = sel4_async_single_threaded_executor::LocalPool::new();
    spawn_async!(pool, exception::waiting_and_handle());
    spawn_async!(pool, exception::waiting_for_end());
    loop {
        let _ = pool.run_all_until_stalled();
    }
}
