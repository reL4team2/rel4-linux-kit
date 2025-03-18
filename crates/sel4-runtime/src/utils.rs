//! 运行函数工具函数
//!
//!
use core::sync::atomic::{AtomicUsize, Ordering};

use common::consts::DEFAULT_PARENT_EP;
use sel4::{
    CNodeCapData, UserContext,
    cap::{SmallPage, Tcb},
    init_thread::slot,
};

/// 创建一个线程
///
/// - `func`  需要执行的函数 `fn()`
/// - `sp`    新线程使用的 栈
/// - `tcb`   [Tcb] 创建线程使用的 Capability
/// - `ipc_addr` IpcBuffer 使用的物理地址，需要 4k 对齐
/// - `ipc_cap`  IpcBuffer 使用的物理页
/// - `args`  参数列表
///
/// TODO: 使用参数列表传递参数
pub fn create_thread(
    func: fn() -> !,
    sp: usize,
    tcb: Tcb,
    ipc_addr: usize,
    ipc_cap: SmallPage,
    _args: &[&str],
) -> Result<(), sel4::Error> {
    let mut ctx = UserContext::default();
    *ctx.pc_mut() = crate::entry::secondary_entry as usize as _;
    *ctx.sp_mut() = sp as _;
    *ctx.c_param_mut(0) = func as usize as u64;
    *ctx.c_param_mut(1) = ipc_addr as _;
    *ctx.c_param_mut(2) = 0; // argc
    *ctx.c_param_mut(3) = 0; // argv
    tcb.tcb_configure(
        DEFAULT_PARENT_EP.cptr(),
        slot::CNODE.cap(),
        CNodeCapData::new(0, 0),
        slot::VSPACE.cap(),
        ipc_addr as _,
        ipc_cap,
    )?;
    tcb.tcb_set_sched_params(slot::TCB.cap(), 0, 255)?;
    tcb.tcb_write_all_registers(true, &mut ctx)
}

/// 申请一个空闲的地址
///
/// - `size` 需要申请的地址块的大小
pub fn alloc_free_addr(size: usize) -> usize {
    static FREE_SIZE: AtomicUsize = AtomicUsize::new(config::SHARE_PAGE_START);
    FREE_SIZE.fetch_add(size, Ordering::SeqCst)
}
