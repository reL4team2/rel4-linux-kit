//! 处理 sel4 任务运行过程中产生的异常
//!
//! 这个模块主要负责处理由当前任务运行的子任务产生的异常,且当前任务的子任务
//! 为传统宏内核应用。目前传统宏内核应用的 syscall 需要预处理，将 syscall 指令
//! 更换为 `0xdeadbeef` 指令，这样在异常处理时可以区分用户异常和系统调用。且不用
//! 为宏内核支持引入多余的部件。
use common::config::PAGE_SIZE;
use sel4::{
    Fault, MessageInfo, UserException, VmFault, cap::Notification, init_thread, with_ipc_buffer,
};
use spin::Lazy;

use crate::{child_test::TASK_MAP, syscall::handle_syscall, utils::obj::alloc_notification};

/// 全局通知
///
/// 在各种结构上绑定的 [Notification]
pub static GLOBAL_NOTIFY: Lazy<Notification> = Lazy::new(alloc_notification);

/// 处理用户异常
///
/// - `tid` 是用户进程绑定的任务 ID
/// - `vmfault` 是发生的错误，包含错误信息
///
/// 函数描述：
/// - 异常指令为 0xdeadbeef 时，说明是系统调用
/// - 异常指令为其他值时，说明是用户异常
pub async fn handle_user_exception(tid: u64, exception: UserException) {
    let task = TASK_MAP.lock().get(&tid).unwrap().clone();

    let ins = task.read_ins(exception.inner().get_FaultIP() as _);

    // 如果是某个特定的指令，则说明此次调用是系统调用
    if Some(0xdeadbeef) == ins {
        let mut user_ctx = task
            .tcb
            .tcb_read_all_registers(true)
            .expect("can't read task context");
        let result = handle_syscall(&task, &mut user_ctx).await;
        debug!("\t SySCall Ret: {:x?}", result);
        let ret_v = match result {
            Ok(v) => v,
            Err(e) => -(e.into_raw() as isize) as usize,
        };

        *user_ctx.gpr_mut(0) = ret_v as _;
        *user_ctx.pc_mut() += 4;

        if task.exit.lock().is_some() {
            return;
        }

        // 写入返回值信息
        task.tcb
            .tcb_write_all_registers(false, &mut user_ctx)
            .unwrap();

        // 检查信号
        task.check_signal(&mut user_ctx);

        if task.exit.lock().is_some() {
            return;
        }

        // 恢复任务运行状态
        task.tcb.tcb_resume().unwrap();
    } else {
        log::debug!("trigger fault: {:#x?}", exception);
    }
}

/// 处理内存异常问题
///
/// - `tid` 是用户进程绑定的任务 ID
/// - `vmfault` 是发生的错误，包含错误信息
pub fn handle_vmfault(tid: u64, vmfault: VmFault) {
    log::warn!("trigger fault: {:#x?}", vmfault);
    let vaddr = vmfault.addr() as usize / PAGE_SIZE * PAGE_SIZE;
    let mut task_map = TASK_MAP.lock();
    let task = task_map.get_mut(&tid).unwrap();
    task.map_blank_page(vaddr);

    task.tcb.tcb_resume().unwrap();
    drop(task_map);
}

// /// 处理 vcpu Fault
// ///
// /// # 参数
// /// - `tid` 用户进程绑定的任务 ID
// /// - `vcpufault` [VCpuFault] 是发生的错误包含错误信息
// ///
// /// Note: 目前仅处理发生 BreakPoint 的情况，根据 HSR 指令进行检测
// ///
// /// # 工具
// ///
// /// - `ESR Decoder` <https://esr.arm64.dev/>
// /// - `ESR Manual` <https://developer.arm.com/documentation/ddi0601/2025-03/AArch64-Registers/ESR-EL2--Exception-Syndrome-Register--EL2->
// pub fn handle_vcpu_fault(tid: u64, vcpufault: VCpuFault) {
//     log::debug!("vcpu fault {:#x?}", vcpufault);
//     let task = TASK_MAP.lock().get(&tid).unwrap().clone();
//     let mut user_ctx = task
//         .tcb
//         .tcb_read_all_registers(true)
//         .expect("can't read task context");
//     let hsr = vcpufault.hsr();
//     // 产生异常的原因
//     let ec = (hsr >> 26) & 0x3f;
//     // 指令长度，0: 16bit 1: 32bit
//     let il = (hsr >> 25) & 0x1;
//     // 执行 BreakPoint 指令，且指令长度为 32bit
//     if ec == 0b111100 && il == 1 {
//         *user_ctx.pc_mut() += 4;
//         task.add_signal(SignalNum::TRAP, task.tid);
//     } else {
//         log::error!("Unhandled fault: {:#x?}", vcpufault);
//         log::error!("{:#x?}", user_ctx);

//         panic!("unspecific fault")
//     }
//     task.check_signal(&mut user_ctx);
//     task.tcb
//         .tcb_write_all_registers(true, &mut user_ctx)
//         .unwrap();
// }

/// 循环等待并处理异常
pub async fn waiting_and_handle(tid: u64, message: MessageInfo) {
    assert!(message.label() < 8, "Unexpected IPC Message");

    let fault = with_ipc_buffer(|buffer| Fault::new(buffer, &message));
    match fault {
        Fault::VmFault(vmfault) => handle_vmfault(tid, vmfault),
        Fault::UserException(ue) => handle_user_exception(tid, ue).await,
        // Fault::VCpuFault(vcf) => handle_vcpu_fault(tid, vcf),
        _ => {
            log::error!("Unhandled fault: {:#x?}", fault);
            let task = TASK_MAP.lock().get(&tid).unwrap().clone();
            let ctx = task.tcb.tcb_read_all_registers(true).unwrap();
            log::error!("{:#x?}", ctx);
        }
    }
}

/// 初始化 exception
///
/// 将 [GLOBAL_NOTIFY] 绑定在 TCB 上
pub fn init() {
    init_thread::slot::TCB
        .cap()
        .tcb_bind_notification(*GLOBAL_NOTIFY)
        .unwrap();
}
