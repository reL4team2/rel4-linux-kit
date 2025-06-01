//! 入口函数和相关配置信息
//!
//!
extern crate sel4_panicking;

use common::config::DEFAULT_EMPTY_SLOT_INDEX;
use core::{hint::spin_loop, ptr};
use sel4_ctors_dtors::run_ctors;
use sel4_kit::ipc_buffer::init_ipc_buffer;
use sel4_panicking::catch_unwind;
use sel4_panicking_env::abort;

sel4_panicking_env::register_debug_put_char!(sel4::sys::seL4_DebugPutChar);

/// 主要的 rust 入口
///
/// 任务在启动的时候首先进入 rust
/// 的入口，在这个函数中将会进行初始化，初始化完成之后会调用真正的入口函数。
#[unsafe(export_name = "sel4_runtime_rust_entry")]
unsafe extern "C" fn main_entry() -> ! {
    unsafe extern "Rust" {
        fn _impl_main();
    }
    let cont_fn = |_| {
        init_ipc_buffer();
        run_ctors();

        // 初始化 slot-manager
        common::slot::init(DEFAULT_EMPTY_SLOT_INDEX..usize::MAX, None);
        // crate::init_log!(log::LevelFilter::Debug);
        common::slot::init_recv_slot();

        match catch_unwind(|| unsafe {
            _impl_main();
            loop {
                spin_loop();
            }
        }) {
            Ok(never) => never,
            Err(_) => {
                abort!("[BlockThread] main() panicked")
            }
        }
    };

    unsafe { sel4_runtime_common::initialize_tls_on_stack_and_continue(cont_fn, ptr::null_mut()) }
}
