use core::{future::Future, pin::{pin, Pin}, task::Poll};

use arm_pl011::pl011::Pl011Uart;
use sel4::{
    cap::{IrqHandler, Notification},
    with_ipc_buffer_mut,
};

pub struct Pl011Poller {
    inner: Pl011Uart,
    noti: Notification,
    irq_handler: IrqHandler,
}

impl Pl011Poller {
    pub fn new(pl011_addr: usize, noti: Notification, irq_handler: IrqHandler) -> Self {
        let mut inner = Pl011Uart::new(pl011_addr as _);
        inner.init();
        inner.ack_interrupts();
        Self {
            inner,
            noti,
            irq_handler,
        }
    }
}

impl Future for Pl011Poller {
    type Output = u8;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let (_, badge) = with_ipc_buffer_mut(|ib| ib.inner_mut().seL4_Poll(self.noti.bits()));
        match badge {
            0 => Poll::Pending,
            _ => {
                let char = self.inner.getchar().unwrap();
                self.inner.ack_interrupts();
                self.irq_handler.irq_handler_ack().unwrap();
                Poll::Ready(char)
            }
        }
    }
}
