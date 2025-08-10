//! 宏内核线程服务，这个线程可以将传统宏内核程序作为子程序运行，可以为子程序提供文件系统、设备等服务。
//! 目前还需要对需要运行的子程序进行预处理。
#![no_std]
#![no_main]
#![deny(missing_docs)]
#![deny(warnings)]
#![feature(never_type)]
#![feature(extract_if)]
#![feature(const_trait_impl)]

use ::fs::file::File;
use common::{config::DEFAULT_SERVE_EP, root::shutdown};
use futures::task::LocalSpawnExt;
use libc_core::fcntl::OpenFlags;

use crate::{
    child_test::TASK_MAP,
    consts::task::{VDSO_AREA_SIZE, VDSO_KADDR},
    timer::handle_timer,
    utils::{blk::get_blk_dev, obj::OBJ_ALLOCATOR},
};

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
pub mod vdso;

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
        drop(data);
    }};
}

const DEF_HEAP_SIZE: usize = 0x380_0000;

sel4_runtime::define_heap!(DEF_HEAP_SIZE);

#[sel4_runtime::main]
fn main() {
    common::slot::init_slot_edge_handler(|slot| {
        OBJ_ALLOCATOR.extend_slot(slot);
    });

    // 初始化 LOG
    logging::init();

    // 初始化 object allocator
    utils::obj::init();

    // 初始化文件系统
    ::fs::dentry::mount_fs(ext4fs::Ext4FileSystem::new(get_blk_dev()), "/");
    ::fs::dentry::mount_fs(allocfs::AllocFS::new(), "/tmp");
    ::fs::dentry::mount_fs(fs::devfs::DevFS::new(), "/dev");
    ::fs::dentry::mount_fs(allocfs::AllocFS::new(), "/var");
    ::fs::dentry::mount_fs(allocfs::AllocFS::new(), "/dev/shm");

    // 初始化设备
    device::init();

    // 初始化异常处理 Mixed IPC/Notification
    exception::init();

    // 初始化定时器
    timer::init();

    {
        vdso::init_vdso_addr();
        let vdso = File::open("/vdso.so", OpenFlags::RDONLY).unwrap();
        let vdso_size = vdso
            .read(unsafe { core::slice::from_raw_parts_mut(VDSO_KADDR as _, VDSO_AREA_SIZE) })
            .unwrap();
        assert!(vdso_size > 0);
    }

    // test_task!("./pipe");
    test_task!("busybox", "sh", "/init.sh");
    // test_task!("./runtest.exe", "-w", "entry-static.exe", "fdopen");
    // test_task!("busybox", "sh", "/iozone_testcode.sh");
    // test_task!("busybox", "sh", "/lmbench_testcode.sh");
    // test_task!("./libc-bench");
    // test_task!("busybox", "which", "ls");
    // test_task!("entry-static.exe", "clock_gettime");
    // test_task!("busybox", "sh", "/run-static.sh");

    let mut pool = sel4_async_single_threaded_executor::LocalPool::new();
    let spawner = pool.spawner();
    loop {
        {
            // 所有的任务都执行完毕
            if !TASK_MAP.lock().iter().any(|x| x.1.exit.lock().is_none()) {
                sel4::debug_println!("\n\n **** rel4-linux-kit **** \nsystem run done😸🎆🎆🎆");
                shutdown();
            }
        }
        let (message, tid) = DEFAULT_SERVE_EP.recv(());
        match tid {
            u64::MAX => handle_timer(),
            _ => spawner
                .spawn_local(exception::waiting_and_handle(tid, message))
                .unwrap(),
        };
        let _ = pool.run_all_until_stalled();
    }
}
