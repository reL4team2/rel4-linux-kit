//! 定时器
//!
//!

use core::future::poll_fn;

use common::arch::{get_curr_ns, get_cval_ns};
use sel4::{CapRights, cap::Notification};
use sel4_kit::ipc::poll_notification;
use slot_manager::LeafSlot;
use spin::Lazy;

use crate::{
    child_test::TASK_MAP,
    utils::obj::{alloc_notification, alloc_slot},
};

static TIMER_IRQ_SLOT: Lazy<LeafSlot> = Lazy::new(alloc_slot);
static TIMER_IRQ_NOTIFY: Lazy<Notification> = Lazy::new(alloc_notification);

/// 创建一个辅助任务来处理时钟等任务
pub async fn aux_thread() {
    sel4::debug_println!("boot aux thread");
    loop {
        wait_timer_irq().await;
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
    }
}

/// 初始化定时器相关的任务
pub fn init() {
    // 将 Notification 移动到 0 号位置，然后通过 mint 赋值并移动回来。
    LeafSlot::from_cap(*TIMER_IRQ_NOTIFY)
        .move_to(LeafSlot::new(0))
        .unwrap();
    LeafSlot::new(0)
        .mint_to(LeafSlot::from_cap(*TIMER_IRQ_NOTIFY), CapRights::all(), 1)
        .unwrap();
    LeafSlot::new(0).delete().unwrap();

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

/// 等待时钟中断的到来
pub async fn wait_timer_irq() {
    poll_fn(|cx| {
        let res = poll_notification(*TIMER_IRQ_NOTIFY);
        if res.is_pending() {
            cx.waker().wake_by_ref();
        }
        res
    })
    .await;
    common::arch::set_timer(0);
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
