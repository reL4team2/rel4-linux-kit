#![no_std]
#![no_main]

extern crate alloc;
extern crate uart_thread;

use common::consts::{DEFAULT_SERVE_EP, REG_LEN};
use sel4::{MessageInfoBuilder, with_ipc_buffer_mut};
use srv_gate::uart::UartIfaceEvent;
use uart_thread::PL011DRV;

#[sel4_runtime::main]
fn main() {
    log::info!("Booting...");
    // let mut pl011 = Pl011UartIfaceImpl::new(VIRTIO_MMIO_VIRT_ADDR);
    let mut pl011 = PL011DRV.lock();

    with_ipc_buffer_mut(|ib| {
        loop {
            let (msg, badge) = DEFAULT_SERVE_EP.recv(());
            log::warn!("recv msg: {:?}", msg);
            match badge {
                // u64::MAX => irq_callback(),
                _ => {
                    let msg_label = match UartIfaceEvent::try_from(msg.label()) {
                        Ok(label) => label,
                        Err(_) => continue,
                    };
                    match msg_label {
                        UartIfaceEvent::init => {
                            sel4::reply(ib, MessageInfoBuilder::default().build());
                        }
                        UartIfaceEvent::getchar => {
                            let c = pl011.getchar();
                            ib.msg_regs_mut()[0] = c as _;
                            sel4::reply(ib, MessageInfoBuilder::default().length(1).build());
                        }
                        UartIfaceEvent::putchar => {
                            pl011.putchar(ib.msg_bytes()[0]);
                            sel4::reply(ib, MessageInfoBuilder::default().build());
                        }
                        UartIfaceEvent::puts => {
                            log::debug!("putstring");
                            let len = ib.msg_regs()[0] as usize;
                            pl011.puts(&ib.msg_bytes()[REG_LEN..len + REG_LEN]);
                            sel4::reply(ib, MessageInfoBuilder::default().build());
                        }
                    }
                }
            }
        }
    });
    // srv_gate::event::handle_events();
    loop {}
}
