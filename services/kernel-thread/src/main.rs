#![no_std]
#![no_main]
#![feature(never_type)]
#![feature(const_trait_impl)]

extern crate alloc;
extern crate sel4_panicking;

mod arch;
mod child_test;
mod device;
mod fs;
mod logging;
mod runtime;
mod syscall;
mod task;
mod thread;
mod utils;

use crate_consts::GRANULE_SIZE;
use sel4::{debug_println, init_thread::slot};
use utils::{init_free_page_addr, FreePagePlaceHolder};

sel4_panicking_env::register_debug_put_char!(sel4::sys::seL4_DebugPutChar);

/// Get the virtual address of the page seat.
pub fn page_seat_vaddr() -> usize {
    unsafe { init_free_page_addr() }
}

/// free page placeholder
pub(crate) static mut FREE_PAGE_PLACEHOLDER: FreePagePlaceHolder =
    FreePagePlaceHolder([0; GRANULE_SIZE]);

fn main() -> ! {
    common::init_recv_slot();
    // 初始化 LOG
    logging::init();

    // 初始化 object allocator
    utils::obj::init();

    // 初始化文件系统
    fs::init();

    // 初始化设备
    device::init();

    // test_func!("[KernelThread] Test Thread", {
    //     child_test::test_child().unwrap();
    // });
    child_test::test_child().unwrap();
    debug_println!("[KernelThread] Say Goodbye");
    slot::TCB.cap().tcb_suspend().unwrap();
    unreachable!()
}
