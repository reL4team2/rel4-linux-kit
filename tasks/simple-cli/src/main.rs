#![no_std]
#![no_main]

use alloc::{string::String, vec::Vec};
use common::services::{fs::FileSerivce, root::find_service, uart::UartService};
use sel4::{MessageInfoBuilder, cap::Endpoint, debug_print, debug_println};
use spin::Lazy;

extern crate alloc;

static FS_SERVICE: Lazy<FileSerivce> = Lazy::new(|| find_service("fs-thread").unwrap().into());
static UART_SERVICE: Lazy<UartService> = Lazy::new(|| find_service("uart-thread").unwrap().into());
static KERNEL_SERVICE: Lazy<Endpoint> = Lazy::new(|| find_service("kernel-thread").unwrap().into());

sel4_runtime::entry_point!(main);

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
        "start-kernel" => {
            // 和 kernel-thread 的 exception::waiting_for_start 结合
            KERNEL_SERVICE.send(MessageInfoBuilder::default().label(0x1234).build());
        }
        "" => {}
        cmd => {
            debug_println!("Can't find command {}", cmd);
        }
    }
}

fn main() -> ! {
    common::init_log!(log::LevelFilter::Trace);
    common::init_recv_slot();

    log::debug!("Starting...");

    FS_SERVICE.ping().unwrap();
    UART_SERVICE.ping().unwrap();

    loop {
        debug_print!("> ");
        let mut str = Vec::new();
        loop {
            let char = UART_SERVICE.getchar().unwrap();
            debug_print!("{}", char::from_u32(char as _).unwrap());

            match char {
                b'\n' | b'\r' => break,
                _ => str.push(char),
            }
        }
        command(&String::from_utf8(str).unwrap());
    }
}
