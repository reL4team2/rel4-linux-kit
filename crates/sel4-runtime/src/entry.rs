//! 入口函数和相关配置信息
//!
//!
extern crate sel4_panicking;

use crate::consts::*;
use core::{mem::transmute, ptr};
use crate_consts::DEFAULT_EMPTY_SLOT_INDEX;
use sel4::{IpcBuffer, set_ipc_buffer};
use sel4_ctors_dtors::run_ctors;
use sel4_dlmalloc::{StaticDlmallocGlobalAlloc, StaticHeap};
use sel4_kit::ipc_buffer::init_ipc_buffer;
use sel4_panicking::catch_unwind;
use sel4_panicking_env::abort;
use sel4_runtime_common::ContArg;
use sel4_sync::PanickingRawMutex;

sel4_panicking_env::register_debug_put_char!(sel4::sys::seL4_DebugPutChar);

#[unsafe(export_name = "__sel4_runtime_common__stack_bottom")]
static STACK_TOP: usize = 0x1_0000_0000;

#[global_allocator]
static GLOBAL_ALLOCATOR: StaticDlmallocGlobalAlloc<
    PanickingRawMutex,
    &'static StaticHeap<HEAP_SIZE>,
> = StaticDlmallocGlobalAlloc::new(PanickingRawMutex::new(), &STATIC_HEAP);

static STATIC_HEAP: StaticHeap<HEAP_SIZE> = StaticHeap::new();

unsafe extern "Rust" {
    fn _impl_main() -> !;
    static _end: usize;
}

#[unsafe(export_name = "sel4_runtime_rust_entry")]
unsafe extern "C" fn main_entry() -> ! {
    let cont_fn = |_| {
        init_ipc_buffer();
        run_ctors();

        // 初始化 slot-manager
        common::slot::init(DEFAULT_EMPTY_SLOT_INDEX..usize::MAX);

        match catch_unwind(|| unsafe { _impl_main() }) {
            Ok(never) => never,
            Err(_) => {
                abort!("[BlockThread] main() panicked")
            }
        }
    };

    unsafe { sel4_runtime_common::initialize_tls_on_stack_and_continue(cont_fn, ptr::null_mut()) }
}

/// 非主线程启动后的入口函数
///
/// - `handler` 初始化之后使用的入口函数
/// - `ib`      [IpcBuffer] 使用的虚拟地址，需要 4k 对齐
/// - `argc`    参数数量
/// - `args`    参数列表，需要指向一个有效的参数地址
pub unsafe extern "C" fn secondary_entry(
    handler: usize,
    ib: *const IpcBuffer,
    argc: usize,
    argv: *const usize,
) -> ! {
    let mut args = [handler, ib as _, argc, argv as _];
    let const_fn = |arg_addr: *mut ContArg| -> ! {
        unsafe {
            let inner_args = core::slice::from_raw_parts_mut(arg_addr as *mut usize, 4);
            set_ipc_buffer((inner_args[1] as *mut IpcBuffer).as_mut().unwrap());
            transmute::<usize, fn()>(inner_args[0])();
        }
        unreachable!()
    };
    unsafe {
        sel4_runtime_common::initialize_tls_on_stack_and_continue(
            const_fn,
            args.as_mut_ptr().cast(),
        )
    };
}

/// 在服务或任务中声明完成初始化后的程序入口
#[macro_export]
macro_rules! entry_point {
    ($main:ident) => {
        #[unsafe(no_mangle)]
        extern "Rust" fn _impl_main() -> ! {
            $main()
        }
    };
}
