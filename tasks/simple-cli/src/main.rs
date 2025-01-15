#![no_std]
#![no_main]

use alloc::{string::String, vec::Vec};
use common::services::{fs::FileSerivce, root::RootService, uart::UartService};
use crate_consts::DEFAULT_PARENT_EP;
use sel4::{debug_print, debug_println};

extern crate alloc;

mod runtime;

static ROOT_SERVICE: RootService = RootService::from_bits(DEFAULT_PARENT_EP);

const FS_SERVICE: FileSerivce = FileSerivce::from_bits(0x21);
const UART_SERVICE: UartService = UartService::from_bits(0x22);

fn command(cmd: &str) {
    match cmd {
        "help" => {
            debug_println!("Available commands:");
            debug_println!("- help: Display this help message");
            debug_println!("- ls: List files in the specified mountpoint");
        }
        "ls" => {
            FS_SERVICE.read_dir("_").unwrap();
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

    // 探索服务，并尝试 ping
    ROOT_SERVICE
        .find_service("fs-thread", FS_SERVICE.leaf_slot())
        .unwrap();
    ROOT_SERVICE
        .find_service("uart-thread", UART_SERVICE.leaf_slot())
        .unwrap();
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
