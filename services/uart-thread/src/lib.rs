#![no_std]
#![feature(used_with_arg)]

extern crate alloc;

use arm_pl011::pl011::Pl011Uart;
use common::{
    config::{SERIAL_DEVICE_IRQ, VIRT_PL011_ADDR},
    root::{register_irq, register_notify},
    slot::alloc_slot,
};
use sel4::{
    Cap, MessageInfo,
    cap_type::{IrqHandler, Notification},
    debug_println,
};
use sel4_kit::slot_manager::LeafSlot;
use srv_gate::{def_event_handler, def_uart_impl, uart::UartIface};

def_uart_impl!(PL011DRV, Pl011UartIfaceImpl::new(VIRT_PL011_ADDR));
def_event_handler!(PL011_IRQ, usize::MAX, irq_handler);

fn irq_handler(msg: &MessageInfo, badge: u64) {
    log::debug!("receive {} from {}", msg.label(), badge);
    // let char = PL011.lock().getchar().unwrap();
    // PL011.lock().ack_interrupts();
    // IRQ_HANDLER.irq_handler_ack().unwrap();
    // let mut ipc_saver = IPC_SAVER.lock();
    // if ipc_saver.queue_len() > 0 {
    //     with_ipc_buffer_mut(|ib| {
    //         ib.msg_bytes_mut()[0] = char;
    //         ipc_saver.reply_one(REV_MSG.length(1).build()).unwrap();
    //     });
    // } else {
    //     BUFFER.lock().push_back(char);
    // }
}

pub struct Pl011UartIfaceImpl {
    device: Pl011Uart,
    notify: Cap<Notification>,
    irq_handler: Cap<IrqHandler>,
}

impl Pl011UartIfaceImpl {
    pub fn new(addr: usize) -> Self {
        debug_println!("create new pl011 iface impl");
        let mut device = Pl011Uart::new(addr as _);
        let notify = alloc_slot().cap();
        let irq_handler = alloc_slot().cap();
        // 向 root-task 申请一个中断
        register_irq(SERIAL_DEVICE_IRQ, LeafSlot::from_cap(irq_handler));

        // 向 root-task 申请一个通知
        register_notify(LeafSlot::from_cap(notify), usize::MAX)
            .expect("Can't register interrupt handler");

        // 设置 pl011 地址空间
        device.init();
        device.ack_interrupts();

        irq_handler.irq_handler_set_notification(notify).unwrap();
        irq_handler.irq_handler_ack().unwrap();

        // 将 Notification 绑定在 TCB 上,以便在接受 IPC 的时候也可以接受 notify
        // sel4::init_thread::slot::TCB
        //     .cap()
        //     .tcb_bind_notification(notify)
        //     .unwrap();

        Self {
            device,
            notify,
            irq_handler,
        }
    }
}

unsafe impl Sync for Pl011UartIfaceImpl {}
unsafe impl Send for Pl011UartIfaceImpl {}

impl UartIface for Pl011UartIfaceImpl {
    fn init(&mut self) {}

    fn putchar(&mut self, c: u8) {
        self.device.putchar(c);
    }

    fn getchar(&mut self) -> u8 {
        self.notify.wait();
        let char = self.device.getchar().unwrap();
        self.device.ack_interrupts();
        self.irq_handler.irq_handler_ack().unwrap();
        char
    }

    fn puts(&mut self, bytes: &[u8]) {
        for &c in bytes {
            self.putchar(c);
        }
    }
}
