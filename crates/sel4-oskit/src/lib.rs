//! sel4-runtime 库
//!
//! 为 sel4/rel4 提供普通任务的基础运行环境，包含环境初始化和自定义库的初始化
#![no_std]
#![deny(warnings)]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
pub mod allocator;
pub mod asm;
#[cfg(feature = "alloc")]
pub mod capset;
pub mod config;
pub mod ipc;
pub mod irq;
#[cfg(feature = "alloc")]
pub mod memory;
