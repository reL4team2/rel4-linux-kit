#![no_std]
#![no_main]

extern crate alloc;

use alloc::collections::vec_deque::VecDeque;
use arm_pl011::pl011::Pl011Uart;
use common::{
    ipc::ipc_saver::IpcSaver,
    services::{
        root::{register_irq, register_notify},
        uart::UartEvent,
    },
    slot::alloc_slot,
    VIRTIO_MMIO_VIRT_ADDR,
};
use crate_consts::{DEFAULT_SERVE_EP, SERIAL_DEVICE_IRQ};
use sel4::{with_ipc_buffer_mut, MessageInfoBuilder};
use sel4_kit::ipc::{poll_endpoint, poll_notification};

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
    register_notify(ntfn.into(), 2).expect("Can't register interrupt handler");

    // 设置 pl011 地址空间
    let mut pl011 = Pl011Uart::new(VIRTIO_MMIO_VIRT_ADDR as _);
    pl011.init();
    pl011.ack_interrupts();

    irq_handler.irq_handler_set_notification(ntfn).unwrap();
    irq_handler.irq_handler_ack().unwrap();

    let rev_msg = MessageInfoBuilder::default();
    let mut buffer = VecDeque::new();
    let mut ipc_saver = IpcSaver::new();

    loop {
        if let Some(_) = poll_notification(ntfn) {
            let char = pl011.getchar().unwrap();
            pl011.ack_interrupts();
            irq_handler.irq_handler_ack().unwrap();

            if ipc_saver.queue_len() > 0 {
                with_ipc_buffer_mut(|ib| {
                    ib.msg_bytes_mut()[0] = char as u8;
                    ipc_saver.reply_one(rev_msg.length(1).build()).unwrap();
                });
            } else {
                buffer.push_back(char);
            }
            // if let Some(cap_slot) = queue.pop_front() {
            //     let ep: Endpoint = cap_slot.into();
            //     with_ipc_buffer_mut(|ib| {
            //         ib.msg_bytes_mut()[0] = char as u8;
            //         ep.send(rev_msg.length(1).build());
            //     });
            //     cap_slot.delete().unwrap();
            // } else {
            //     buffer.push_back(char);
            // }
        }

        if let Some((msg, _badge)) = poll_endpoint(DEFAULT_SERVE_EP) {
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
                        ib.msg_bytes_mut()[0] = c as u8;
                        sel4::reply(ib, rev_msg.length(1).build());
                    }),
                    None => ipc_saver.save_caller().unwrap(),
                },
            }
        }
    }
}
