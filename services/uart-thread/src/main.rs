#![no_std]
#![no_main]

extern crate alloc;
extern crate uart_thread;

use common::consts::{DEFAULT_SERVE_EP, REG_LEN};
use sel4::{MessageInfoBuilder, with_ipc_buffer, with_ipc_buffer_mut};
use srv_iface::uart::{UartIface, UartIfaceEvent};
use uart_thread::PL011DRV;

sel4_runtime::entry_point!(main);

// Questions:
// 1. 修改了调用逻辑，让本来不需要 Mutex 的都需要全局加锁
// 2. 可能存在 Endpoint 和 Notification 复用一个通道的情况，这时候该如何分发
// 3. 有些代码会存在需要保存调用者的情况，比如 uart 的 getchar，然后再有数据之后再分发，这个时候这个函数模型不太对劲。
//    可能可以用 Future 解决这个问题。但是  Impl 不能存在于 extern "Rust" 中。GG
//    所以不能使用 IPC 共享函数
// 4. 利用 rust 的 trait object 来实现 IPC 的分发

fn main() -> ! {
    common::init_log!(log::LevelFilter::Debug);
    common::init_recv_slot();

    log::info!("Booting...");
    // let mut pl011 = Pl011UartIfaceImpl::new(VIRTIO_MMIO_VIRT_ADDR);
    let mut pl011 = PL011DRV.lock();

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
                        with_ipc_buffer_mut(|ib| {
                            sel4::reply(ib, MessageInfoBuilder::default().build());
                        });
                    }
                    UartIfaceEvent::getchar => {
                        let c = pl011.getchar();
                        with_ipc_buffer_mut(|ib| {
                            ib.msg_regs_mut()[0] = c as _;
                            sel4::reply(ib, MessageInfoBuilder::default().length(1).build());
                        })
                    }
                    UartIfaceEvent::putchar => {
                        with_ipc_buffer(|ib| {
                            pl011.putchar(ib.msg_bytes()[0]);
                        });
                        with_ipc_buffer_mut(|ib| {
                            sel4::reply(ib, MessageInfoBuilder::default().build());
                        });
                    }
                    UartIfaceEvent::puts => {
                        log::debug!("putstring");
                        with_ipc_buffer(|ib| {
                            let len = ib.msg_regs()[0] as usize;
                            pl011.puts(&ib.msg_bytes()[REG_LEN..len + REG_LEN]);
                        });
                        with_ipc_buffer_mut(|ib| {
                            sel4::reply(ib, MessageInfoBuilder::default().build());
                        })
                    }
                }
            }
        }
    }
}
