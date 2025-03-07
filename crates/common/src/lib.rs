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
pub mod thread;

pub use obj_allocator::*;
pub use utils::*;

// FIXME: Make this variable more generic.
pub const VIRTIO_MMIO_ADDR: usize = 0xa003e00;
pub const PL011_ADDR: usize = 0x0900_0000;
pub const VIRTIO_MMIO_VIRT_ADDR: usize = 0x1_2000_0000;

const VIRTIO_BLK_OFFSET: usize = 0x3e00;
const VIRTIO_NET_OFFSET: usize = 0x3c00;

pub const VIRTIO_MMIO_BLK_VIRT_ADDR: usize = VIRTIO_MMIO_VIRT_ADDR + VIRTIO_BLK_OFFSET;
pub const VIRTIO_MMIO_NET_VIRT_ADDR: usize = VIRTIO_MMIO_VIRT_ADDR + VIRTIO_NET_OFFSET;
