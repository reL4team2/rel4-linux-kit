use alloc::sync::Arc;
use libc_core::{
    internal::SigAction,
    signal::{SignalNum, UContext},
    types::SigSet,
};
use sel4::UserContext;
use spin::Mutex;
use syscalls::Errno;
use zerocopy::{FromBytes, FromZeros};

use crate::child_test::futex_signal_task;

use super::Sel4Task;

pub struct TaskSignal {
    /// 程序结束时发出的信号
    pub exit_sig: Option<SignalNum>,
    /// 信号屏蔽位
    pub mask: SigSet,
    /// 信号处理函数
    pub actions: Arc<Mutex<[SigAction; 65]>>,
    /// 等待处理的信号
    pub pedings: SigSet,
}

impl Default for TaskSignal {
    fn default() -> Self {
        Self {
            exit_sig: None,
            mask: SigSet::default(),
            actions: Arc::new(Mutex::new([const { SigAction::empty() }; 65])),
            pedings: SigSet::empty(),
        }
    }
}

impl Sel4Task {
    /// 检查当前任务是否有信号待处理
    ///
    /// - `ctx` [UserContext] 当前任务的上下文
    ///
    /// ## 说明
    /// 当前任务的上下文在检测到信号待处理的时候会进程存储
    pub fn check_signal(&self, ctx: &mut UserContext) {
        let mut task_signal: spin::MutexGuard<'_, TaskSignal> = self.signal.lock();
        let sigmask = task_signal.mask;
        if let Some(signal) = task_signal.pedings.pop_one(Some(sigmask)) {
            // 保存处理信号前的上下文，信号处理结束后恢复
            let actions = task_signal.actions.lock();
            let action = actions[signal.num()].clone();
            drop(actions);

            if action.handler == SigAction::SIG_IGN {
                // ignore signal if the handler of is SIG_IGN(1)
                return;
            } else if action.handler == 0 || action.handler == SigAction::SIG_DFL {
                // if there doesn't have signal handler.
                // Then use default handler. Exit or do nothing.
                if matches!(signal, SignalNum::CANCEL | SignalNum::SEGV | SignalNum::ILL) {
                    drop(task_signal);
                    self.exit_with(signal.num() as u32);
                }
                return;
            }
            let new_sp = self.write_ucontext(ctx, task_signal.mask);
            task_signal.mask = action.mask;
            *ctx.c_param_mut(0) = signal.num() as _;
            *ctx.c_param_mut(1) = 0;
            *ctx.c_param_mut(2) = new_sp as _;

            *ctx.pc_mut() = action.handler as _;
            *ctx.gpr_mut(30) = action.restorer as _;
            *ctx.sp_mut() = new_sp as _;

            self.tcb.tcb_write_all_registers(false, ctx).unwrap();
            task_signal.pedings.remove(signal);
        }
    }

    /// 添加信号
    ///
    /// ## 参数
    /// - `signal` 为当前 [Sel4Task] 添加的信号
    /// - `from`   从哪个线程发送的
    ///
    /// ## 说明
    ///
    /// 添加信号可能会打断某些行为
    #[inline]
    pub fn add_signal(&self, signal: SignalNum, from: usize) {
        self.signal.lock().pedings.insert(signal);
        futex_signal_task(self.futex_table.clone(), self.tid, Errno::EINTR);

        if from != self.tid {
            let mut ctx = self.tcb.tcb_read_all_registers(true).unwrap();
            self.check_signal(&mut ctx);
            self.tcb.tcb_resume().unwrap();
        }
    }

    /// 将 [UserContext] 写入栈
    pub fn write_ucontext(&self, ctx: &UserContext, mask: SigSet) -> usize {
        let mut uctx = UContext::new_zeroed();
        uctx.sig_mask.sigset = mask;
        uctx.regs.pc = *ctx.pc() as _;
        uctx.regs.sp = *ctx.sp() as _;
        for i in 0..31 {
            uctx.regs.gregs[i] = *ctx.gpr(i) as _;
        }
        // let bytes = uctx.as_bytes();
        // let new_sp = uctx.regs.sp - uctx.as_bytes().len();
        let new_sp = (uctx.regs.sp - size_of::<UContext>()) & !0xF;
        let bytes = unsafe {
            core::slice::from_raw_parts(
                &uctx as *const UContext as *const u8,
                size_of::<UContext>(),
            )
        };
        self.write_bytes(new_sp, bytes);
        new_sp
    }

    /// 从当前栈中读取 sig
    pub fn read_ucontext(&self, ctx: &mut UserContext) {
        let sig_sp = *ctx.sp() as usize;
        let uctx_bytes = self.read_bytes(sig_sp, size_of::<UContext>()).unwrap();
        let uctx = UContext::read_from_bytes(&uctx_bytes).unwrap();
        self.signal.lock().mask = uctx.sig_mask.sigset;
        *ctx.pc_mut() = uctx.regs.pc as _;
        *ctx.sp_mut() = uctx.regs.sp as _;
        for i in 0..31 {
            *ctx.gpr_mut(i) = uctx.regs.gregs[i] as _;
        }
    }
}
