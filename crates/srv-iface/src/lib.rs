#![no_std]

pub mod uart;

use linkme::distributed_slice;
use uart::UartIface;

#[distributed_slice]
pub static UART_IMPLS: [fn(&mut dyn UartIface)];
