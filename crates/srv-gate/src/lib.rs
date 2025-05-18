#![no_std]
#![feature(used_with_arg)]

extern crate alloc;

pub mod __prelude;
pub mod blk;
pub mod consts;
pub mod event;
pub mod uart;

use blk::BlockIface;
pub use linkme;
pub use paste::paste;

use core::fmt::Write;

use alloc::sync::Arc;
use linkme::distributed_slice;
use spin::{Lazy, Mutex};
use uart::UartIface;

#[distributed_slice]
pub static UART_IMPLS: [Lazy<Arc<Mutex<dyn UartIface>>>];
#[distributed_slice]
pub static BLK_IMPLS: [Lazy<Arc<Mutex<dyn BlockIface>>>];

/// 定义一个事件处理器，利用 paste! 将 中断处理号加入到名称中用于做标识，防止 irq 冲突
#[macro_export]
macro_rules! def_uart_impl {
    ($name:ident, $f:expr) => {
        #[$crate::linkme::distributed_slice($crate::UART_IMPLS)]
        #[linkme(crate = $crate::linkme)]
        #[unsafe(no_mangle)]
        pub static $name: spin::Lazy<alloc::sync::Arc<spin::Mutex<dyn $crate::uart::UartIface>>> =
            spin::Lazy::new(|| alloc::sync::Arc::new(spin::Mutex::new($f)));
    };
}

/// 定义一个事件处理器，利用 paste! 将 中断处理号加入到名称中用于做标识，防止 irq 冲突
#[macro_export]
macro_rules! def_blk_impl {
    ($name:ident, $f:expr) => {
        #[$crate::linkme::distributed_slice($crate::BLK_IMPLS)]
        #[linkme(crate = $crate::linkme)]
        #[unsafe(no_mangle)]
        pub static $name: spin::Lazy<alloc::sync::Arc<spin::Mutex<dyn $crate::blk::BlockIface>>> =
            spin::Lazy::new(|| alloc::sync::Arc::new(spin::Mutex::new($f)));
    };
}

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
