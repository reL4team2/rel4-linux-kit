//! sel4-kit
//!
//! sel4-kit 是针对 sel4 和 rust-sel4 的再封装，主要是为了优化代码
//! 并且提高代码的可读性，以及对一些不太合理的地方进行重写。

#![no_std]
#![deny(warnings)]
#![deny(missing_docs)]

pub mod arch;
pub mod ipc;
pub mod ipc_buffer;
