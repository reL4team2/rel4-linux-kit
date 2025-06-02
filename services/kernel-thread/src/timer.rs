//! 定时器
//!
//!

use core::time::Duration;

use sel4::{CapRights, cap::Notification};
use sel4_kit::{
    arch::{GENERIC_TIMER_PCNT_IRQ, current_time, get_cval, set_timer},
    slot_manager::LeafSlot,
};
use spin::Lazy;

use crate::{child_test::TASK_MAP, exception::GLOBAL_NOTIFY, utils::obj::alloc_slot};

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

/// 处理时钟中断
///
///
pub fn handle_timer() {
    let mut task_map = TASK_MAP.lock();
    // 处理已经到时间的定时器
    let curr_time = current_time();
    task_map.values_mut().for_each(|task| {
        if task.exit.is_none() && !task.timer.is_zero() && curr_time > task.timer {
            task.timer = Duration::ZERO;
            task.tcb.tcb_resume().unwrap();
        }
    });
    // 设置下一个定时器
    let next_time = task_map
        .values()
        .filter(|x| x.exit.is_none() && !x.timer.is_zero())
        .map(|x| x.timer)
        .min()
        .unwrap_or(Duration::ZERO);
    set_timer(next_time);
    drop(task_map);
    TIMER_IRQ_SLOT
        .cap::<sel4::cap_type::IrqHandler>()
        .irq_handler_ack()
        .unwrap();
}

/// 刷新定时器
///
/// 当对一个新的定时器设置了定时状态后，就会刷新定时器内容。然后将最新的值写入到定时器中。遍历所有的睡眠状态，找到最小的时间，然后设置定时器。
pub fn flush_timer(next: Duration) {
    let cval = get_cval();
    if next < cval || cval.is_zero() {
        set_timer(next);
    }
}
