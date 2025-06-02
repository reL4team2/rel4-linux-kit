use crate::__prelude::*;
use common::ipc_trait;

#[ipc_trait(event = UART_EVENT)]
pub trait UartIface: Sync + Send {
    fn init(&mut self);
    fn putchar(&mut self, c: u8);
    fn getchar(&mut self) -> u8;
    fn puts(&mut self, bytes: &[u8]);
}

#[cfg(uart_ipc)]
mod _impl {

    use common::{generate_ipc_send, root::find_service};
    use sel4::cap::Endpoint;

    use crate::def_uart_impl;

    use super::{UartIface, UartIfaceEvent};

    def_uart_impl!(UART_IPC, UartIfaceIPCImpl {
        ep: find_service("uart-thread").unwrap().into(),
    });

    pub struct UartIfaceIPCImpl {
        ep: Endpoint,
    }

    // #[ipc_trait_impl]
    impl UartIface for UartIfaceIPCImpl {
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
}
