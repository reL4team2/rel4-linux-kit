use common::ipc_trait;

#[ipc_trait]
pub trait UartIface: Sync + Send {
    fn init(&mut self);
    fn putchar(&mut self, c: u8);
    fn getchar(&mut self) -> u8;
    fn puts(&mut self, bytes: &[u8]);
}

#[cfg(feature = "uart-ipc")]
mod _impl {
    use alloc::sync::Arc;
    use common::{generate_ipc_send, services::root::find_service};
    use sel4::cap::Endpoint;
    use spin::{Lazy, Mutex};

    use super::{UartIface, UartIfaceEvent};

    #[linkme::distributed_slice(super::super::UART_IMPLS)]
    static PL011DRV: Lazy<Arc<Mutex<dyn super::UartIface>>> = Lazy::new(|| {
        Arc::new(Mutex::new(UartIfaceTest {
            ep: find_service("uart-thread").unwrap().into(),
        }))
    });

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
}
