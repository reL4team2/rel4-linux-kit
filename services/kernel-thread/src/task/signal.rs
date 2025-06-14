use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use libc_core::{internal::SigAction, signal::SignalNum, types::SigSet};
use sel4::UserContext;

use super::Sel4Task;

pub struct TaskSignal {
    /// 程序结束时发出的信号
    pub exit_sig: u32,
    /// 信号屏蔽位
    pub mask: SigSet,
    /// 信号处理函数
    pub actions: [Option<SigAction>; 65],
    /// 等待处理的信号
    pub pedings: VecDeque<SignalNum>,
    /// 处理信号过程中存储的上下文
    pub save_context: Vec<UserContext>,
}

impl Default for TaskSignal {
    fn default() -> Self {
        Self {
            exit_sig: Default::default(),
            mask: SigSet::default(),
            actions: [const { None }; 65],
            pedings: VecDeque::new(),
            save_context: Vec::new(),
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
        warn!("check signal is not checking the mask now");
        let mut task_signal = self.signal.lock();
        if let Some(signal) = task_signal.pedings.pop_front() {
            // 保存处理信号前的上下文，信号处理结束后恢复
            task_signal.save_context.push(ctx.clone());

            let action = match &task_signal.actions[signal.num()] {
                Some(action) => action,
                None => {
                    warn!("signal {:?} is not handled", signal);
                    return;
                }
            };
            log::warn!("action {signal:?} value: {:#x?}", action);
            *ctx.c_param_mut(0) = signal as _;
            *ctx.pc_mut() = action.handler as _;
            *ctx.gpr_mut(30) = action.restorer as _;

            self.tcb.tcb_write_all_registers(false, ctx).unwrap();
        }
    }

    /// 添加信号
    ///
    /// ## 参数
    /// - `signal` 为当前 [Sel4Task] 添加的信号
    ///
    /// ## 说明
    ///
    /// 添加信号可能会打断某些行为
    #[inline]
    pub fn add_signal(&self, signal: SignalNum) {
        self.signal.lock().pedings.push_back(signal);
    }
}
