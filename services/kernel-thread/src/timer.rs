//! 定时器
//!
//!

use common::arch::{get_curr_ns, get_cval_ns};
use sel4::{CapRights, cap::Notification};
use slot_manager::LeafSlot;
use spin::Lazy;

use crate::{child_test::TASK_MAP, exception::GLOBAL_NOTIFY, utils::obj::alloc_slot};

static TIMER_IRQ_SLOT: Lazy<LeafSlot> = Lazy::new(|| alloc_slot());
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
    common::services::root::register_irq(common::arch::GENERIC_TIMER_PCNT_IRQ, *TIMER_IRQ_SLOT)
        .unwrap();
    TIMER_IRQ_SLOT
        .cap::<sel4::cap_type::IrqHandler>()
        .irq_handler_set_notification(*TIMER_IRQ_NOTIFY)
        .unwrap();
    // 设置初始的值，并响应中断
    common::arch::set_timer(0);
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
    let curr_ns = get_curr_ns();
    task_map.values_mut().for_each(|task| {
        if task.exit.is_none() && task.timer != 0 && curr_ns > task.timer {
            task.timer = 0;
            task.tcb.tcb_resume().unwrap();
        }
    });
    // 设置下一个定时器
    let next_ns = task_map
        .values()
        .filter(|x| x.exit.is_none() && x.timer != 0)
        .map(|x| x.timer)
        .min()
        .unwrap_or(0);
    common::arch::set_timer(next_ns);
    drop(task_map);
    TIMER_IRQ_SLOT
        .cap::<sel4::cap_type::IrqHandler>()
        .irq_handler_ack()
        .unwrap();
}

/// 刷新定时器
///
/// 当对一个新的定时器设置了定时状态后，就会刷新定时器内容。然后将最新的值写入到定时器中。遍历所有的睡眠状态，找到最小的时间，然后设置定时器。
pub fn flush_timer(next_ns: usize) {
    let cval_ns = get_cval_ns();
    if next_ns < cval_ns || cval_ns == 0 {
        common::arch::set_timer(next_ns);
    }
}
