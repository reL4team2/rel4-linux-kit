//! sel4-runtime 库
//!
//! 为 sel4/rel4 提供普通任务的基础运行环境，包含环境初始化和自定义库的初始化
#![no_std]
#![deny(warnings)]
#![deny(missing_docs)]

#[cfg(not(feature = "root-task"))]
pub mod entry;
#[cfg(not(feature = "root-task"))]
pub mod heap;
pub mod logging;
pub mod macros;
pub mod utils;

pub use sel4_logging::{Logger, LoggerBuilder};
