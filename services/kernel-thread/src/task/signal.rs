use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use sel4::UserContext;

use crate::syscall::types::signal::{SigAction, SigProcMask};

use super::Sel4Task;

pub struct TaskSignal {
    /// 程序结束时发出的信号
    pub exit_sig: u32,
    /// 信号屏蔽位
    pub mask: SigProcMask,
    /// 信号处理函数
    pub actions: [Option<SigAction>; 65],
    /// 等待处理的信号
    pub pedings: VecDeque<u8>,
    /// 处理信号过程中存储的上下文
    pub save_context: Vec<UserContext>,
}

impl Default for TaskSignal {
    fn default() -> Self {
        Self {
            exit_sig: Default::default(),
            mask: SigProcMask::default(),
            actions: [None; 65],
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
    pub fn check_signal(&mut self, ctx: &mut UserContext) {
        warn!("check signal is not checking the mask now");
        if let Some(signal) = self.signal.pedings.pop_front() {
            // 保存处理信号前的上下文，信号处理结束后恢复
            self.signal.save_context.push(ctx.clone());

            let action = match self.signal.actions[signal as usize] {
                Some(action) => action,
                None => {
                    warn!("signal {} is not handled", signal);
                    return;
                }
            };
            log::warn!("action {signal} value: {:#x?}", action);
            *ctx.c_param_mut(0) = signal as _;
            *ctx.pc_mut() = action.handler as _;
            *ctx.gpr_mut(30) = action.restorer as _;

            self.tcb.tcb_write_all_registers(false, ctx).unwrap();
        }
    }
}
