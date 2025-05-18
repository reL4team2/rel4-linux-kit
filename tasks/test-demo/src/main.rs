#![no_std]
#![no_main]
#![feature(used_with_arg)]

extern crate alloc;
#[cfg(not(feature = "uart-ipc"))]
extern crate uart_thread;

use srv_gate::println;

#[sel4_runtime::main]
fn main() {
    println!("Hello World!");
}
