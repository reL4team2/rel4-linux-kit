//! 提供架构相关的 sel4 功能支持补充
//!
//! 为 rust-sel4 提供补充

#[cfg(target_arch = "aarch64")]
mod aarch64;

#[cfg(target_arch = "aarch64")]
pub use aarch64::*;
