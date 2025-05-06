#![no_std]
#![no_main]

extern crate alloc;
extern crate uart_thread;

use common::{
    consts::{DEFAULT_SERVE_EP, REG_LEN},
    services::uart::UartEvent,
};
use sel4::{MessageInfoBuilder, with_ipc_buffer, with_ipc_buffer_mut};
use uart_thread::{irq_callback, putchar};

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

    uart_thread::init_uart();

    loop {
        let (msg, badge) = DEFAULT_SERVE_EP.recv(());
        log::warn!("recv msg: {:?}", msg);
        match badge {
            u64::MAX => irq_callback(),
            _ => {
                let msg_label = match UartEvent::try_from(msg.label()) {
                    Ok(label) => label,
                    Err(_) => continue,
                };
                match msg_label {
                    UartEvent::Ping => {
                        uart_thread::ping();
                        with_ipc_buffer_mut(|ib| {
                            sel4::reply(ib, MessageInfoBuilder::default().build());
                        });
                    }
                    UartEvent::GetChar => todo!(),
                    UartEvent::PutChar => {
                        log::debug!("putchar");
                        with_ipc_buffer(|ib| {
                            let c = ib.msg_bytes()[0];
                            putchar(c);
                        });
                    }
                    UartEvent::PutString => {
                        log::debug!("putstring");
                        with_ipc_buffer(|ib| {
                            let len = ib.msg_regs()[0] as usize;
                            let s = ib.msg_bytes()[REG_LEN..len + REG_LEN].as_ref();
                            uart_thread::puts(s);
                        });
                    }
                }
            }
        }
    }
}
