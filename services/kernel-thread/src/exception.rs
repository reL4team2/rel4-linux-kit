//! 处理 sel4 任务运行过程中产生的异常
//!
//! 这个模块主要负责处理由当前任务运行的子任务产生的异常,且当前任务的子任务
//! 为传统宏内核应用。目前传统宏内核应用的 syscall 需要预处理，将 syscall 指令
//! 更换为 `0xdeadbeef` 指令，这样在异常处理时可以区分用户异常和系统调用。且不用
//! 为宏内核支持引入多余的部件。
use common::page::PhysPage;
use crate_consts::{DEFAULT_SERVE_EP, PAGE_SIZE};
use sel4::{with_ipc_buffer, Fault, UserException, VmFault};

use crate::{child_test::TASK_MAP, syscall::handle_syscall, utils::obj::alloc_page};

/// 处理用户异常
///
/// - `tid` 是用户进程绑定的任务 ID
/// - `vmfault` 是发生的错误，包含错误信息
///
/// 函数描述：
/// - 异常指令为 0xdeadbeef 时，说明是系统调用
/// - 异常指令为其他值时，说明是用户异常
pub fn handle_user_exception(tid: u64, exception: UserException) {
    let mut task_map = TASK_MAP.lock();
    let task = task_map.get_mut(&tid).unwrap();

    let ins = task.read_ins(exception.inner().get_FaultIP() as _);

    // 如果是某个特定的指令，则说明此次调用是系统调用
    if Some(0xdeadbeef) == ins {
        let mut user_ctx = task
            .tcb
            .tcb_read_all_registers(true)
            .expect("can't read task context");
        let result = handle_syscall(task, &mut user_ctx);
        debug!("\t SySCall Ret: {:x?}", result);
        let ret_v = match result {
            Ok(v) => v,
            Err(e) => -(e.into_raw() as isize) as usize,
        };
        *user_ctx.gpr_mut(0) = ret_v as _;
        *user_ctx.pc_mut() = user_ctx.pc().wrapping_add(4) as _;
        task.tcb
            .tcb_write_all_registers(true, &mut user_ctx)
            .unwrap();
    }
}

/// 处理内存异常问题
///
/// - `tid` 是用户进程绑定的任务 ID
/// - `vmfault` 是发生的错误，包含错误信息
pub fn handle_vmfault(tid: u64, vmfault: VmFault) {
    let vaddr = vmfault.addr() as usize / PAGE_SIZE * PAGE_SIZE;
    let page_cap = PhysPage::new(alloc_page());
    let mut task_map = TASK_MAP.lock();
    let task = task_map.get_mut(&tid).unwrap();
    task.map_page(vaddr, page_cap);

    task.tcb.tcb_resume().unwrap();
    drop(task_map);
}

/// 循环等待并处理异常
pub fn waiting_and_handle() -> ! {
    loop {
        let (message, tid) = DEFAULT_SERVE_EP.recv(());

        assert!(message.label() < 8, "Unexpected IPC Message");

        let fault = with_ipc_buffer(|buffer| Fault::new(&buffer, &message));
        match fault {
            Fault::VmFault(vmfault) => handle_vmfault(tid, vmfault),
            Fault::UserException(ue) => handle_user_exception(tid, ue),
            _ => {
                log::error!("Unhandled fault: {:#x?}", fault);
            }
        }

        sel4::r#yield();
    }
}
