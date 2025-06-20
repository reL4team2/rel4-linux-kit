#![no_std]
#![no_main]
#![feature(used_with_arg)]

extern crate alloc;
#[cfg(not(uart_ipc))]
extern crate uart_thread;

use srv_gate::println;

sel4_runtime::define_heap!(common::config::SERVICE_HEAP_SIZE);

#[sel4_runtime::main]
fn main() {
    println!("Hello World!");
}
