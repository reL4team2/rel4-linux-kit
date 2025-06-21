//! 定时器
//!
//!

use core::{
    task::{Poll, Waker},
    time::Duration,
};

use alloc::vec::Vec;
use common::slot::alloc_slot;
use sel4::{CapRights, cap::Notification};
use sel4_kit::{
    arch::{GENERIC_TIMER_PCNT_IRQ, current_time, set_timer},
    slot_manager::LeafSlot,
};
use spin::{Lazy, Mutex};
use syscalls::Errno;

use crate::{child_test::TASK_MAP, exception::GLOBAL_NOTIFY, task::PollWakeEvent};

static TIMER_IRQ_SLOT: Lazy<LeafSlot> = Lazy::new(alloc_slot);
static TIMER_IRQ_NOTIFY: Lazy<Notification> = Lazy::new(|| {
    // 从 Global_Notify 复制一个具有 badge 的Notification
    let slot = alloc_slot();
    LeafSlot::from_cap(*GLOBAL_NOTIFY)
        .mint_to(slot, CapRights::all(), usize::MAX)
        .unwrap();
    slot.cap()
});

/// 初始化定时器相关的任务
pub fn init() {
    // 注册 Timer IRQ
    common::root::register_irq(GENERIC_TIMER_PCNT_IRQ, *TIMER_IRQ_SLOT);
    TIMER_IRQ_SLOT
        .cap::<sel4::cap_type::IrqHandler>()
        .irq_handler_set_notification(*TIMER_IRQ_NOTIFY)
        .unwrap();
    // 设置初始的值，并响应中断
    set_timer(Duration::ZERO);
    TIMER_IRQ_SLOT
        .cap::<sel4::cap_type::IrqHandler>()
        .irq_handler_ack()
        .unwrap();
}

enum TimerType {
    /// (tid, Waker)
    WaitTime(usize, Waker),
    /// (pid)
    ITimer(usize),
}

/// 时间等待队列 (目标时间，任务 id, Waker)
static TIME_QUEUE: Mutex<Vec<(Duration, TimerType)>> = Mutex::new(Vec::new());

/// 处理时钟中断
///
///
pub fn handle_timer() {
    // 处理已经到时间的定时器
    let curr_time = current_time();

    TIME_QUEUE.lock().retain(|(duration, timer_ty)| {
        if curr_time >= *duration {
            match timer_ty {
                TimerType::WaitTime(_tid, waker) => waker.wake_by_ref(),
                TimerType::ITimer(pid) => handle_process_timer(curr_time, *pid),
            };
        }
        curr_time < *duration
    });

    // 设置下一个定时器
    let next = TIME_QUEUE
        .lock()
        .first()
        .map(|x| x.0)
        .unwrap_or(Duration::ZERO);
    set_timer(next);

    TIMER_IRQ_SLOT
        .cap::<sel4::cap_type::IrqHandler>()
        .irq_handler_ack()
        .unwrap();
}

/// 等待时间到达
pub struct WaitForTime(pub Duration, usize);

impl Future for WaitForTime {
    type Output = Result<(), Errno>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        if current_time() > self.0 {
            return Poll::Ready(Ok(()));
        }

        let waker = cx.waker().clone();
        let curr_task = TASK_MAP.lock().get(&(self.1 as _)).unwrap().clone();

        // 如果被 Signal 打断
        if matches!(
            curr_task.waker.lock().take(),
            Some((PollWakeEvent::Signal(_), _))
        ) {
            return Poll::Ready(Err(Errno::EINTR));
        }
        *curr_task.waker.lock() = Some((PollWakeEvent::Blocking, cx.waker().clone()));

        TIME_QUEUE
            .lock()
            .push((self.0, TimerType::WaitTime(self.1, waker)));
        TIME_QUEUE
            .lock()
            .sort_by(|(dura_a, ..), (dura_b, ..)| dura_a.cmp(dura_b));
        set_timer(TIME_QUEUE.lock().first().unwrap().0);
        Poll::Pending
    }
}

/// 等待时间
///
/// ## 参数
/// - `duration` 目标等待的时间
/// - `tid`      等待的线程 id
pub async fn wait_time(duration: Duration, tid: usize) -> Result<usize, Errno> {
    WaitForTime(duration, tid).await?;
    Ok(0)
}

/// 设置进程定时器
pub fn set_process_timer(pid: usize, next: Duration) {
    TIME_QUEUE.lock().push((next, TimerType::ITimer(pid)));
    TIME_QUEUE
        .lock()
        .sort_by(|(dura_a, ..), (dura_b, ..)| dura_a.cmp(dura_b));
    log::debug!(
        "set next timer curr: {:?}  next: {:?}",
        current_time(),
        next
    );
    set_timer(TIME_QUEUE.lock().first().unwrap().0);
}

/// 处理进程 Timer 时间
pub fn handle_process_timer(curr_time: Duration, pid: usize) {
    log::debug!("handle process tiemr: {:?}, pid: {}", curr_time, pid);
    let _ = TASK_MAP
        .lock()
        .values()
        .find(|x| x.pid == pid)
        .inspect(|x| x.add_signal(libc_core::signal::SignalNum::ALRM, pid));
}
