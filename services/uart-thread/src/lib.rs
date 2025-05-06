#![no_std]

extern crate alloc;

use alloc::collections::vec_deque::VecDeque;
use arm_pl011::pl011::Pl011Uart;
use common::{
    ipc::ipc_saver::IpcSaver,
    services::root::{register_irq, register_notify},
    slot::alloc_slot,
};
use config::{SERIAL_DEVICE_IRQ, VIRTIO_MMIO_VIRT_ADDR};
use sel4::{
    Cap, MessageInfoBuilder,
    cap_type::{IrqHandler, Notification},
    init_thread, with_ipc_buffer_mut,
};
use slot_manager::LeafSlot;
use spin::{Lazy, Mutex};

static BUFFER: Mutex<VecDeque<u8>> = Mutex::new(VecDeque::new());
static REV_MSG: Lazy<MessageInfoBuilder> = Lazy::new(MessageInfoBuilder::default);
static IPC_SAVER: Mutex<IpcSaver> = Mutex::new(IpcSaver::new());
static PL011: Mutex<Pl011Uart> = Mutex::new(Pl011Uart::new(VIRTIO_MMIO_VIRT_ADDR as _));
static IRQ_HANDLER: Lazy<Cap<IrqHandler>> = Lazy::new(|| alloc_slot().cap());
static NTFN: Lazy<Cap<Notification>> = Lazy::new(|| alloc_slot().cap());

pub fn init_uart() {
    // 向 root-task 申请一个中断
    register_irq(SERIAL_DEVICE_IRQ, LeafSlot::from_cap(*IRQ_HANDLER));

    // 向 root-task 申请一个通知
    register_notify(LeafSlot::from_cap(*NTFN), usize::MAX)
        .expect("Can't register interrupt handler");

    // 设置 pl011 地址空间
    PL011.lock().init();
    PL011.lock().ack_interrupts();

    IRQ_HANDLER.irq_handler_set_notification(*NTFN).unwrap();
    IRQ_HANDLER.irq_handler_ack().unwrap();

    init_thread::slot::TCB
        .cap()
        .tcb_bind_notification(*NTFN)
        .unwrap();
}

pub fn gechar() -> Option<u8> {
    BUFFER.lock().pop_front()
}

pub fn putchar(c: u8) {
    PL011.lock().putchar(c);
}

pub fn puts(s: &[u8]) {
    for &c in s {
        putchar(c);
    }
}

pub fn ping() {
    log::debug!("Service Received Ping");
}

pub fn irq_callback() {
    let char = PL011.lock().getchar().unwrap();
    PL011.lock().ack_interrupts();
    IRQ_HANDLER.irq_handler_ack().unwrap();

    let mut ipc_saver = IPC_SAVER.lock();
    if ipc_saver.queue_len() > 0 {
        with_ipc_buffer_mut(|ib| {
            ib.msg_bytes_mut()[0] = char;
            ipc_saver.reply_one(REV_MSG.length(1).build()).unwrap();
        });
    } else {
        BUFFER.lock().push_back(char);
    }
}
