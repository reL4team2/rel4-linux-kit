#![no_std]
#![no_main]

extern crate alloc;
extern crate uart_thread;

use alloc::{string::String, vec::Vec};
use sel4::{debug_print, debug_println};
use srv_gate::UART_IMPLS;

fn command(cmd: &str) {
    match cmd {
        "help" => {
            debug_println!("Available commands:");
            debug_println!("- help: Display this help message");
            debug_println!("- ls: List files in the specified mountpoint");
            debug_println!("- start-kernel: Start User Kernel To Execute Linux App");
        }
        "ls" => {
            unimplemented!("read_dir is unimplemented")
        }
        // "start-kernel" => {
        //     // 和 kernel-thread 的 exception::waiting_for_start 结合
        //     KERNEL_SERVICE.send(MessageInfoBuilder::default().label(0x1234).build());
        // }
        "" => {}
        cmd => {
            debug_println!("Can't find command {}", cmd);
        }
    }
}

#[sel4_runtime::main]
fn main() {
    log::debug!("Starting...");

    // FS_SERVICE.ping().unwrap();
    UART_IMPLS[0].lock().init();
    loop {
        debug_print!("> ");
        let mut str = Vec::new();
        loop {
            let char = UART_IMPLS[0].lock().getchar();
            debug_print!("{}", char::from_u32(char as _).unwrap());

            match char {
                b'\n' | b'\r' => break,
                _ => str.push(char),
            }
        }
        command(&String::from_utf8(str).unwrap());
    }
}
