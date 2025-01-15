#![no_std]
#![no_main]

extern crate alloc;

use arm_pl011::pl011;
use common::{
    services::{root::RootService, uart::UartServiceLabel},
    VIRTIO_MMIO_VIRT_ADDR,
};
use crate_consts::{DEFAULT_CUSTOM_SLOT, DEFAULT_PARENT_EP, DEFAULT_SERVE_EP, SERIAL_DEVICE_IRQ};
use sel4::{
    cap::{Endpoint, IrqHandler, Notification},
    init_thread::slot::CNODE,
    with_ipc_buffer_mut, MessageInfoBuilder,
};

mod runtime;

static ROOT_SERVICE: RootService = RootService::from_bits(DEFAULT_PARENT_EP);

fn main() -> ! {
    common::init_log!(log::LevelFilter::Trace);
    common::init_recv_slot();

    log::info!("Booting...");

    let serve_ep = Endpoint::from_bits(DEFAULT_SERVE_EP);

    let mut pl011 = pl011::Pl011Uart::new(VIRTIO_MMIO_VIRT_ADDR as _);
    pl011.ack_interrupts();
    pl011.init();

    // Register interrupt handler and notification
    // Allocate irq handler
    let irq_handler = IrqHandler::from_bits(DEFAULT_CUSTOM_SLOT + 1);
    ROOT_SERVICE
        .register_irq(SERIAL_DEVICE_IRQ, CNODE.cap().absolute_cptr(irq_handler))
        .expect("can't register interrupt handler");

    // Allocate notification
    let ntfn = Notification::from_bits(DEFAULT_CUSTOM_SLOT);
    ROOT_SERVICE
        .alloc_notification(CNODE.cap().absolute_cptr(ntfn))
        .expect("Can't register interrupt handler");

    irq_handler.irq_handler_set_notification(ntfn).unwrap();
    irq_handler.irq_handler_ack().unwrap();

    let rev_msg = MessageInfoBuilder::default();
    loop {
        let (message, _) = serve_ep.recv(());
        let msg_label = match UartServiceLabel::try_from(message.label()) {
            Ok(label) => label,
            Err(_) => continue,
        };
        match msg_label {
            UartServiceLabel::Ping => {
                with_ipc_buffer_mut(|ib| {
                    sel4::reply(ib, rev_msg.build());
                });
            }
            UartServiceLabel::GetChar => {
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
    loop {}
}
