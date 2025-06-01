#![no_std]
#![feature(str_from_raw_parts)]

extern crate alloc;

mod obj_allocator;

pub mod arch;
pub mod config;
pub mod ipc_saver;
pub mod ipcrw;
pub mod log_impl;
pub mod page;
pub mod services;
pub mod slot;

pub use common_macros::{generate_ipc_send, ipc_trait, ipc_trait_impl};
pub use obj_allocator::*;

// FIXME: Make this variable more generic.
