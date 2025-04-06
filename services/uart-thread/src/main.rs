#![no_std]
#![no_main]

extern crate alloc;

use alloc::collections::vec_deque::VecDeque;
use arm_pl011::pl011::Pl011Uart;
use common::{
    consts::DEFAULT_SERVE_EP,
    ipc::ipc_saver::IpcSaver,
    services::{
        root::{register_irq, register_notify},
        uart::UartEvent,
    },
    slot::alloc_slot,
};
use config::{SERIAL_DEVICE_IRQ, VIRTIO_MMIO_VIRT_ADDR};
use sel4::{MessageInfoBuilder, init_thread, with_ipc_buffer_mut};

sel4_runtime::entry_point!(main);

fn main() -> ! {
    common::init_log!(log::LevelFilter::Error);
    common::init_recv_slot();

    log::info!("Booting...");

    // 向 root-task 申请一个中断
    let irq_handler = alloc_slot().cap();
    register_irq(SERIAL_DEVICE_IRQ, irq_handler.into()).expect("can't register interrupt handler");

    // 向 root-task 申请一个通知
    let ntfn = alloc_slot().cap();
    register_notify(ntfn.into(), usize::MAX).expect("Can't register interrupt handler");

    // 设置 pl011 地址空间
    let mut pl011 = Pl011Uart::new(VIRTIO_MMIO_VIRT_ADDR as _);
    pl011.init();
    pl011.ack_interrupts();

    irq_handler.irq_handler_set_notification(ntfn).unwrap();
    irq_handler.irq_handler_ack().unwrap();

    let rev_msg = MessageInfoBuilder::default();
    let mut buffer = VecDeque::new();
    let mut ipc_saver = IpcSaver::new();
    init_thread::slot::TCB
        .cap()
        .tcb_bind_notification(ntfn)
        .unwrap();

    loop {
        let (msg, badge) = DEFAULT_SERVE_EP.recv(());
        if badge == u64::MAX {
            let char = pl011.getchar().unwrap();
            pl011.ack_interrupts();
            irq_handler.irq_handler_ack().unwrap();

            if ipc_saver.queue_len() > 0 {
                with_ipc_buffer_mut(|ib| {
                    ib.msg_bytes_mut()[0] = char;
                    ipc_saver.reply_one(rev_msg.length(1).build()).unwrap();
                });
            } else {
                buffer.push_back(char);
            }
        } else {
            let msg_label = match UartEvent::try_from(msg.label()) {
                Ok(label) => label,
                Err(_) => continue,
            };
            match msg_label {
                UartEvent::Ping => {
                    with_ipc_buffer_mut(|ib| {
                        sel4::reply(ib, rev_msg.build());
                    });
                }
                UartEvent::GetChar => match buffer.pop_front() {
                    Some(c) => with_ipc_buffer_mut(|ib| {
                        ib.msg_bytes_mut()[0] = c;
                        sel4::reply(ib, rev_msg.length(1).build());
                    }),
                    None => ipc_saver.save_caller().unwrap(),
                },
            }
        }
    }
}
