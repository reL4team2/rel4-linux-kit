#![no_std]
#![no_main]

extern crate alloc;
extern crate uart_thread;

use common::config::{DEFAULT_SERVE_EP, REG_LEN};
use sel4::{MessageInfoBuilder, with_ipc_buffer_mut};
use srv_gate::uart::UartIfaceEvent;
use uart_thread::PL011DRV;

#[sel4_runtime::main]
fn main() {
    log::info!("Booting...");
    let mut pl011 = PL011DRV.lock();

    with_ipc_buffer_mut(|ib| {
        loop {
            // TODO: 根据 badge 保存 IPC reply，并在需要的时候发回
            let (msg, _) = DEFAULT_SERVE_EP.recv(());
            let msg_label = match UartIfaceEvent::try_from(msg.label()) {
                Ok(label) => label,
                Err(_) => continue,
            };
            match msg_label {
                UartIfaceEvent::init => sel4::reply(ib, MessageInfoBuilder::default().build()),
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
    });
}
