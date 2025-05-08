#![no_std]
#![no_main]
#![feature(used_with_arg)]

extern crate alloc;
extern crate uart_thread;

use alloc::sync::Arc;
use common::{generate_ipc_send, services::root::find_service};
use sel4::cap::Endpoint;
use spin::{Lazy, Mutex};
use srv_iface::{
    println,
    uart::{UartIface, UartIfaceEvent},
};
use zerocopy::IntoBytes;

sel4_runtime::entry_point!(main);

pub struct UartIfaceTest {
    ep: Endpoint,
}

impl UartIface for UartIfaceTest {
    #[generate_ipc_send(label = UartIfaceEvent::init)]
    fn init(&mut self) {
        todo!()
    }

    #[generate_ipc_send(label = UartIfaceEvent::putchar)]
    fn putchar(&mut self, c: u8) {
        todo!()
    }

    #[generate_ipc_send(label = UartIfaceEvent::getchar)]
    fn getchar(&mut self) -> u8 {
        todo!()
    }

    #[generate_ipc_send(label = UartIfaceEvent::puts)]
    fn puts(&mut self, bytes: &[u8]) {
        todo!()
    }
}

// #[linkme::distributed_slice(srv_iface::UART_IMPLS)]
// static PL011DRV: Lazy<Arc<Mutex<dyn UartIface>>> = Lazy::new(|| {
//     Arc::new(Mutex::new(UartIfaceTest {
//         ep: find_service("uart-thread").unwrap().into(),
//     }))
// });

fn main() -> ! {
    common::init_log!(log::LevelFilter::Trace);
    common::init_recv_slot();

    println!("Hello World!");
    loop {}
}
