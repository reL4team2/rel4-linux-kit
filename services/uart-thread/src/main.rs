#![no_std]
#![no_main]

extern crate alloc;

use arm_pl011::pl011;
use common::{
    services::{
        root::{register_irq, register_notify},
        uart::UartEvent,
    },
    VIRTIO_MMIO_VIRT_ADDR,
};
use crate_consts::{DEFAULT_CUSTOM_SLOT, DEFAULT_SERVE_EP, SERIAL_DEVICE_IRQ};
use sel4::{
    cap::{IrqHandler, Notification},
    with_ipc_buffer_mut, MessageInfoBuilder,
};

mod runtime;

fn main() -> ! {
    common::init_log!(log::LevelFilter::Error);
    common::init_recv_slot();

    log::info!("Booting...");

    let mut pl011 = pl011::Pl011Uart::new(VIRTIO_MMIO_VIRT_ADDR as _);
    pl011.ack_interrupts();
    pl011.init();

    // 向 root-task 申请一个中断
    let irq_handler = IrqHandler::from_bits(DEFAULT_CUSTOM_SLOT + 1);
    register_irq(SERIAL_DEVICE_IRQ, irq_handler.into()).expect("can't register interrupt handler");

    // 向 root-task 申请一个通知
    let ntfn = Notification::from_bits(DEFAULT_CUSTOM_SLOT);
    register_notify(ntfn.into()).expect("Can't register interrupt handler");

    irq_handler.irq_handler_set_notification(ntfn).unwrap();
    irq_handler.irq_handler_ack().unwrap();

    let rev_msg = MessageInfoBuilder::default();
    loop {
        let (message, _) = DEFAULT_SERVE_EP.recv(());
        let msg_label = match UartEvent::try_from(message.label()) {
            Ok(label) => label,
            Err(_) => continue,
        };
        match msg_label {
            UartEvent::Ping => {
                with_ipc_buffer_mut(|ib| {
                    sel4::reply(ib, rev_msg.build());
                });
            }
            UartEvent::GetChar => {
                ntfn.wait();
                let char = pl011.getchar().unwrap();
                pl011.ack_interrupts();
                irq_handler.irq_handler_ack().unwrap();
                with_ipc_buffer_mut(|ib| {
                    ib.msg_bytes_mut()[0] = char as u8;
                    sel4::reply(ib, rev_msg.length(1).build());
                });
            }
        }
    }
}
