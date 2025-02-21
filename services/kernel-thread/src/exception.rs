use common::page::PhysPage;
use crate_consts::{DEFAULT_SERVE_EP, PAGE_SIZE};
use sel4::{with_ipc_buffer, Fault, UserException, VmFault};

use crate::{child_test::TASK_MAP, utils::obj::alloc_page};

/// 处理用户异常
///
/// - `tid` 是用户进程绑定的任务 ID
/// - `vmfault` 是发生的错误，包含错误信息
///
/// 函数描述：
/// - 异常指令为 0xdeadbeef 时，说明是系统调用
/// - 异常指令为其他值时，说明是用户异常
pub fn handle_user_exception(tid: u64, exception: UserException) {
    let task_map = TASK_MAP.lock();
    let task = task_map.get(&tid).unwrap();

    let ins = task.read_ins(exception.inner().get_FaultIP() as _);

    // 如果是某个特定的指令，则说明此次调用是系统调用
    if Some(0xdeadbeef) == ins {
        let mut user_ctx = task
            .tcb
            .tcb_read_all_registers(true)
            .expect("can't read task context");
        let syscall_id = user_ctx.gpr_mut(8).clone();
        log::debug!(
            "received syscall id: {:#x} pc: {:#x}",
            syscall_id,
            user_ctx.pc()
        );
        log::debug!("Received user exception");
    }

    // handle_ipc_call(&task, &message, user_exception);
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
