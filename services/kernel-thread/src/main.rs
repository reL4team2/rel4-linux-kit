#![no_std]
#![no_main]
#![feature(never_type)]
#![feature(const_trait_impl)]

extern crate alloc;
extern crate sel4_panicking;

mod child_test;
mod logging;
mod runtime;
mod syscall;
mod task;
mod thread;
mod utils;

use common::{
    services::{fs::FileSerivce, root::RootService, uart::UartService},
    ObjectAllocator,
};
use crate_consts::{
    DEFAULT_CUSTOM_SLOT, DEFAULT_EMPTY_SLOT_INDEX, DEFAULT_PARENT_EP, GRANULE_SIZE,
    KERNEL_THREAD_SLOT_NUMS,
};
use sel4::{debug_println, init_thread::slot, Cap};
use spin::Mutex;
use utils::{init_free_page_addr, FreePagePlaceHolder};

sel4_panicking_env::register_debug_put_char!(sel4::sys::seL4_DebugPutChar);

/// Get the virtual address of the page seat.
pub fn page_seat_vaddr() -> usize {
    unsafe { init_free_page_addr() }
}

/// The object allocator for the kernel thread.
pub(crate) static OBJ_ALLOCATOR: Mutex<ObjectAllocator> = Mutex::new(ObjectAllocator::empty());

/// free page placeholder
pub(crate) static mut FREE_PAGE_PLACEHOLDER: FreePagePlaceHolder =
    FreePagePlaceHolder([0; GRANULE_SIZE]);

const ROOT_SERVICE: RootService = RootService::from_bits(DEFAULT_PARENT_EP);

fn main() -> ! {
    common::init_recv_slot();
    logging::init();

    debug_println!("[KernelThread] EntryPoint");
    OBJ_ALLOCATOR.lock().init(
        DEFAULT_EMPTY_SLOT_INDEX..KERNEL_THREAD_SLOT_NUMS,
        Cap::from_bits(DEFAULT_CUSTOM_SLOT as _),
    );
    debug_println!("[KernelThread] Object Allocator initialized");

    // 寻找 fs_service 并尝试 ping
    let fs_service = FileSerivce::from_leaf_slot(OBJ_ALLOCATOR.lock().allocate_slot());
    ROOT_SERVICE
        .find_service("ext4-thread", fs_service.leaf_slot())
        .unwrap();
    fs_service.ping().unwrap();

    // 寻找 uart_service 并尝试 ping
    let uart_service = UartService::from_leaf_slot(OBJ_ALLOCATOR.lock().allocate_slot());
    ROOT_SERVICE
        .find_service("uart-thread", uart_service.leaf_slot())
        .unwrap();
    uart_service.ping().unwrap();

    test_func!("[KernelThread] Test Thread", {
        let ep = OBJ_ALLOCATOR.lock().alloc_endpoint();
        child_test::test_child(ep).unwrap()
    });
    debug_println!("[KernelThread] Say Goodbye");
    slot::TCB.cap().tcb_suspend().unwrap();
    unreachable!()
}
