//! 宏内核线程服务，这个线程可以将传统宏内核程序作为子程序运行，可以为子程序提供文件系统、设备等服务。
//! 目前还需要对需要运行的子程序进行预处理。
#![no_std]
#![no_main]
#![deny(missing_docs)]
#![deny(warnings)]
#![feature(never_type)]
#![feature(const_trait_impl)]

use futures::task::LocalSpawnExt;
use libc_core::fcntl::OpenFlags;

use crate::utils::blk::get_blk_dev;

#[macro_use]
extern crate log;
#[macro_use]
extern crate alloc;
#[cfg(not(fs_ipc))]
extern crate lwext4_thread;
#[cfg(not(uart_ipc))]
extern crate uart_thread;

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
        let file =
            ::fs::file::File::open(concat!("/", $file), OpenFlags::RDONLY).unwrap();
        let mut data = vec![0u8; file.file_size().unwrap()];
        file.read(&mut data).unwrap();
        child_test::add_test_child(&data, &[$file $(,$args)*]).unwrap();
        sel4::debug_println!("loading file: {}", $file);
    }};
}

#[sel4_runtime::main]
fn main() {
    // 初始化 LOG
    logging::init();

    // 初始化 object allocator
    utils::obj::init();

    // 初始化文件系统
    ::fs::dentry::mount_fs(ext4fs::Ext4FileSystem::new(get_blk_dev()), "/");

    // 初始化设备
    device::init();

    // 初始化异常处理 Mixed IPC/Notification
    exception::init();

    // 初始化定时器
    timer::init();

    // {
    //     let file = ::fs::file::File::open("/busybox", OpenFlags::RDONLY).unwrap();
    //     let mut data = vec![0u8; file.file_size().unwrap()];
    //     file.read(&mut data).unwrap();
    //     child_test::add_test_child(&data, &["echo", "123"]).unwrap();
    // }
    // test_task!("busybox", "sh", "/init.sh");
    test_task!("runtest.exe", "-w", "entry-static.exe", "basename");

    let mut pool = sel4_async_single_threaded_executor::LocalPool::new();
    spawn_async!(pool, exception::waiting_and_handle());
    spawn_async!(pool, exception::waiting_for_end());
    loop {
        let _ = pool.run_all_until_stalled();
    }
}
