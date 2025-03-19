#![no_std]
#![feature(str_from_raw_parts)]

extern crate alloc;

mod obj_allocator;
mod utils;

pub mod arch;
pub mod consts;
pub mod ipc;
pub mod log_impl;
pub mod page;
pub mod services;
pub mod slot;

pub use obj_allocator::*;
pub use utils::*;

// FIXME: Make this variable more generic.
