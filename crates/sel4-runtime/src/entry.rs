//! 入口函数和相关配置信息
//!
//!
extern crate sel4_panicking;

use crate::consts::*;
use core::ptr;
use crate_consts::DEFAULT_EMPTY_SLOT_INDEX;
use sel4_ctors_dtors::run_ctors;
use sel4_dlmalloc::{StaticDlmallocGlobalAlloc, StaticHeap};
use sel4_kit::ipc_buffer::init_ipc_buffer;
use sel4_panicking::catch_unwind;
use sel4_panicking_env::abort;
use sel4_runtime_common::set_eh_frame_finder;
use sel4_sync::PanickingRawMutex;

sel4_panicking_env::register_debug_put_char!(sel4::sys::seL4_DebugPutChar);
sel4_runtime_common::declare_stack!(STACK_SIZE);

#[global_allocator]
static GLOBAL_ALLOCATOR: StaticDlmallocGlobalAlloc<
    PanickingRawMutex,
    &'static StaticHeap<HEAP_SIZE>,
> = StaticDlmallocGlobalAlloc::new(PanickingRawMutex::new(), &STATIC_HEAP);

static STATIC_HEAP: StaticHeap<HEAP_SIZE> = StaticHeap::new();

extern "Rust" {
    fn _impl_main() -> !;
    static _end: usize;
}

#[no_mangle]
unsafe extern "C" fn sel4_runtime_rust_entry() -> ! {
    let cont_fn = |_| {
        #[cfg(panic = "unwind")]
        set_eh_frame_finder().unwrap();

        init_ipc_buffer();
        run_ctors();

        // 初始化 slot-manager
        common::slot::init(DEFAULT_EMPTY_SLOT_INDEX..usize::MAX);

        match catch_unwind(|| _impl_main()) {
            Ok(never) => never,
            Err(_) => {
                abort!("[BlockThread] main() panicked")
            }
        }
    };

    sel4_runtime_common::initialize_tls_on_stack_and_continue(cont_fn, ptr::null_mut())
}

/// 在服务或任务中声明完成初始化后的程序入口
#[macro_export]
macro_rules! entry_point {
    ($main:ident) => {
        #[no_mangle]
        extern "Rust" fn _impl_main() -> ! {
            $main()
        }
    };
}
