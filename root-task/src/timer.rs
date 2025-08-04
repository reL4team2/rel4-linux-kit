use sel4_kit::{
    arch::{GENERIC_TIMER_PCNT_IRQ, current_time, set_timer},
    slot_manager::LeafSlot,
};
use core::time::Duration;
use common::slot::alloc_slot;
use spin::Lazy;
use sel4::cap::Notification;
use crate::GLOBAL_NOTIFY;

static TIMER_IRQ_SLOT: Lazy<LeafSlot> = Lazy::new(alloc_slot);
static TIMER_IRQ_NOTIFY: Lazy<Notification> = Lazy::new(|| {
    // 从 Global_Notify 复制一个具有 badge 的Notification
    let slot = alloc_slot();
    LeafSlot::from_cap(*GLOBAL_NOTIFY)
        .mint_to(slot, sel4::CapRights::all(), u64::MAX as usize)
        .unwrap();
    slot.cap()
});

pub fn init() {
    // 注册 Timer IRQ
    // common::root::register_irq(GENERIC_TIMER_PCNT_IRQ, *TIMER_IRQ_SLOT);
    sel4::init_thread::slot::IRQ_CONTROL
        .cap()
        .irq_control_get(GENERIC_TIMER_PCNT_IRQ as u64, &TIMER_IRQ_SLOT.abs_cptr())
        .unwrap();

    TIMER_IRQ_SLOT
        .cap::<sel4::cap_type::IrqHandler>()
        .irq_handler_set_notification(*TIMER_IRQ_NOTIFY)
        .unwrap();

    sel4::init_thread::slot::TCB
        .cap()
        .tcb_bind_notification(*GLOBAL_NOTIFY)
        .unwrap();

    // 设置初始的值，并响应中断
    set_timer(Duration::ZERO);
    TIMER_IRQ_SLOT
        .cap::<sel4::cap_type::IrqHandler>()
        .irq_handler_ack()
        .unwrap();
}

pub fn timer_ack(duration: Duration) {
    let time = current_time() + duration;
    set_timer(time);
    // 重新设置 IRQ
    TIMER_IRQ_SLOT
        .cap::<sel4::cap_type::IrqHandler>()
        .irq_handler_ack()
        .unwrap();
}

pub fn timer(d: Duration) {
    let time = current_time() + d;
    set_timer(d);
}
