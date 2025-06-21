#![no_std]
#![feature(str_from_raw_parts)]

#[cfg(feature = "alloc")]
extern crate alloc;
mod obj_allocator;

pub mod config;
#[cfg(feature = "alloc")]
pub mod ipc_saver;
pub mod ipcrw;
pub mod log_impl;
pub mod macros;
#[cfg(feature = "alloc")]
pub mod mem;
pub mod page;
pub mod root;
pub mod slot;

pub use common_macros::{generate_ipc_send, ipc_trait, ipc_trait_impl};
pub use obj_allocator::*;

pub use sel4_logging::{Logger, LoggerBuilder};

// FIXME: Make this variable more generic.
