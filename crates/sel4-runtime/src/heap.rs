//! 堆初始化环境
//!
//! 这个模块中定义了程序将会使用的堆栈结构

use common::config;
pub use sel4_dlmalloc::{StaticDlmallocGlobalAlloc, StaticHeap};
pub use sel4_sync::PanickingRawMutex;

/// [sel4_runtime_common] 默认的 start 在启动的时候会将 `__sel4_runtime_common__stack_bottom`
/// 中保存的地址作为栈顶,这里的符号是 __stack_bottom 确实很让人疑惑，可能是 `rust-sel4`
/// 的作者有些不一样的考虑。
#[unsafe(export_name = "__sel4_runtime_common__stack_bottom")]
static STACK_TOP: usize = config::SERVICE_BOOT_STACK_TOP;

/// 定义一个 Heap Allocator
#[macro_export]
macro_rules! define_heap {
    ($size:path) => {
        const HEAP_SIZE: usize = $size;
        /// 服务进程使用的堆，这个堆将会被用来分配内存。
        static STATIC_HEAP: $crate::heap::StaticHeap<HEAP_SIZE> = $crate::heap::StaticHeap::new();

        /// 内存分配器，使用了 [PanickingRawMutex]
        ///
        /// 如果发生了死锁现象，将会 `panic`
        #[global_allocator]
        static GLOBAL_ALLOCATOR: $crate::heap::StaticDlmallocGlobalAlloc<
            $crate::heap::PanickingRawMutex,
            &$crate::heap::StaticHeap<HEAP_SIZE>,
        > = $crate::heap::StaticDlmallocGlobalAlloc::new(
            $crate::heap::PanickingRawMutex::new(),
            &STATIC_HEAP,
        );
    };
}
