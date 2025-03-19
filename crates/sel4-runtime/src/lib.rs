//! sel4-runtime 库
//!
//! 为 sel4/rel4 提供普通任务的基础运行环境，包含环境初始化和自定义库的初始化
#![no_std]
#![deny(warnings)]
#![deny(missing_docs)]

pub mod entry;
pub mod heap;
pub mod utils;
