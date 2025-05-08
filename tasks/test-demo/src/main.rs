#![no_std]
#![no_main]
#![feature(used_with_arg)]

extern crate alloc;
#[cfg(not(feature = "uart-ipc"))]
extern crate uart_thread;

use srv_iface::println;

sel4_runtime::entry_point!(main);

fn main() -> ! {
    common::init_log!(log::LevelFilter::Trace);
    common::init_recv_slot();

    println!("Hello World!");
    loop {}
}
