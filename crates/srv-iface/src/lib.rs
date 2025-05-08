#![no_std]
#![feature(used_with_arg)]

extern crate alloc;

pub mod uart;

use core::fmt::Write;

use alloc::sync::Arc;
use linkme::distributed_slice;
use spin::{Lazy, Mutex};
use uart::UartIface;

#[distributed_slice]
pub static UART_IMPLS: [Lazy<Arc<Mutex<dyn UartIface>>>];

pub struct Console;

impl Write for Console {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        UART_IMPLS[0].lock().puts(s.as_bytes());
        Ok(())
    }
}

pub fn _print(args: core::fmt::Arguments) {
    Console.write_fmt(args).expect("can't print arguments");
}

/// Print macro to print polyhal information with newline
#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($fmt: expr $(, $($arg: tt)+)?) => {
        $crate::print!("{}", format_args!($fmt $(, $($arg)+)?))
    };
}

/// Print macro to print polyhal information with newline
#[macro_export]
macro_rules! print {
    ($fmt: expr $(, $($arg: tt)+)?) => {
        $crate::_print(format_args!($fmt $(, $($arg)+)?))
    };
}
