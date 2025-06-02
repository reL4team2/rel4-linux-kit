#![no_std]
#![no_main]

extern crate alloc;
extern crate uart_thread;

use common::{config::DEFAULT_SERVE_EP, read_types, reply_with};
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
                UartIfaceEvent::getchar => reply_with!(ib, pl011.getchar()),
                UartIfaceEvent::putchar => {
                    pl011.putchar(read_types!(u8));
                    sel4::reply(ib, MessageInfoBuilder::default().build());
                }
                UartIfaceEvent::puts => {
                    pl011.puts(&read_types!(&[u8]));
                    sel4::reply(ib, MessageInfoBuilder::default().build());
                }
            }
        }
    });
}
