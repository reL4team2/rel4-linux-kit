#![no_std]
#![feature(str_from_raw_parts)]

extern crate alloc;

pub mod log_impl;
mod obj_allocator;
pub mod page;
pub mod services;
mod uspace;
mod utils;

use core::cell::UnsafeCell;
use crate_consts::PAGE_SIZE;
pub use obj_allocator::*;
use sel4::{with_ipc_buffer, with_ipc_buffer_mut, MessageInfo};
pub use uspace::*;
pub use utils::*;

// FIXME: Make this variable more generic.
pub const VIRTIO_MMIO_ADDR: usize = 0xa003e00;
pub const PL011_ADDR: usize = 0x0900_0000;

pub const VIRTIO_MMIO_VIRT_ADDR: usize = 0x1_2000_0000;

const VIRTIO_BLK_OFFSET: usize = 0x3e00;

const VIRTIO_NET_OFFSET: usize = 0x3c00;

pub const VIRTIO_MMIO_BLK_VIRT_ADDR: usize = VIRTIO_MMIO_VIRT_ADDR + VIRTIO_BLK_OFFSET;

pub const VIRTIO_MMIO_NET_VIRT_ADDR: usize = VIRTIO_MMIO_VIRT_ADDR + VIRTIO_NET_OFFSET;

/// Impl custom message label quickly.
macro_rules! impl_message_label {
    {
        $(#[$m:meta])*
        pub enum $name:ident : $start:literal {
            $(
                $field:ident $(( $($t:ty),* ))? => $num:literal
            ),* $(,)?
        }
    } => {
        $(#[$m])*
        pub enum $name {
            $($field $(($($t),*))? ),*
        }

        impl $name {

            pub fn try_from(raw_label: usize) -> Option<Self> {
                let label = raw_label - $start;
                match label {
                    $(
                        $num => {
                            todo!()
                        }
                    )*
                    _ => None
                }
            }
        }
    }
}

impl_message_label! {
    #[repr(usize)]
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub enum TestMessageLabel: 100 {
        MessageLabel => 0,
        Test1 => 1,
        Test2(u8,u8) => 2
    }
}

/// Custom Message Label for transfer between tasks.
#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CustomMessageLabel {
    TestCustomMessage = 0,
    SysCall = 1,
    Exit = 2,
}

impl CustomMessageLabel {
    /// The start index of the custom message label
    const LABEL_START: u64 = 0x100;

    /// Try to convert a MessageInfo to a CustomMessageLabel
    pub fn try_from(message: &MessageInfo) -> Option<Self> {
        // Get the true index for the CustomMessageLabel
        let label = match message.label() >= Self::LABEL_START {
            true => message.label() - Self::LABEL_START,
            false => return None,
        };
        // Convert the true index to a CustomMessageLabel enum
        match label {
            0x0 => Some(Self::TestCustomMessage),
            0x1 => Some(Self::SysCall),
            0x2 => Some(Self::Exit),
            _ => None,
        }
    }

    pub fn to_label(&self) -> u64 {
        *self as u64 + Self::LABEL_START
    }
}

pub type IrqNum = u64;

#[repr(usize)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NetRequsetabel {
    New,
    IsNonBlocking(u64),
    SetNonBlocking(u64, u64),
    Bind(u64, u64),
    // id, buf, buf_len
    Send(u64, u64, u64),
    Recv(u64, u64, u64),
    RecvTimeout(u64, u64, u64, u64),
    Connect(u64, u64),
    Listen(u64),
    Accept(u64),
    Shutdown(u64),
    Close(u64),
}

impl NetRequsetabel {
    const LABEL_START: u64 = 0x400;

    /// Try to convert a MessageInfo to a NetRequsetabel
    pub fn try_from(message: &MessageInfo) -> Option<Self> {
        // Get the true index for the NetRequsetabel
        let label = match message.label() >= Self::LABEL_START {
            true => message.label() - Self::LABEL_START,
            false => return None,
        };
        // Convert the true index to a NetRequsetabel enum
        with_ipc_buffer(|buffer| {
            let regs = buffer.msg_regs();
            match label {
                0x0 => Some(Self::New),
                0x1 => Some(Self::IsNonBlocking(regs[0])),
                0x2 => Some(Self::SetNonBlocking(regs[0], regs[1])),
                0x3 => Some(Self::Bind(regs[0], regs[1])),
                0x4 => Some(Self::Send(regs[0], regs[1], regs[2])),
                0x5 => Some(Self::Recv(regs[0], regs[1], regs[2])),
                0x6 => Some(Self::RecvTimeout(regs[0], regs[1], regs[2], regs[3])),
                0x7 => Some(Self::Connect(regs[0], regs[1])),
                0x8 => Some(Self::Listen(regs[0])),
                0x9 => Some(Self::Accept(regs[0])),
                0xa => Some(Self::Shutdown(regs[0])),
                0xb => Some(Self::Close(regs[0])),
                _ => None,
            }
        })
    }

    pub fn to_label(&self) -> u64 {
        let n = match self {
            Self::New => 0,
            Self::IsNonBlocking(_) => 1,
            Self::SetNonBlocking(_, _) => 2,
            Self::Bind(_, _) => 3,
            Self::Send(_, _, _) => 4,
            Self::Recv(_, _, _) => 5,
            Self::RecvTimeout(_, _, _, _) => 6,
            Self::Connect(_, _) => 7,
            Self::Listen(_) => 8,
            Self::Accept(_) => 9,
            Self::Shutdown(_) => 10,
            Self::Close(_) => 11,
        };
        Self::LABEL_START + n
    }

    pub fn build(&self) -> MessageInfo {
        let caps_unwrapped = 0;
        let mut extra_caps = 0;
        let mut msg_size = 0;

        with_ipc_buffer_mut(|buffer| {
            let regs = buffer.msg_regs_mut();
            match self {
                Self::New => {}
                Self::IsNonBlocking(id) => {
                    regs[0] = *id;
                    msg_size = 1;
                }
                Self::SetNonBlocking(id, non_blocking) => {
                    regs[0] = *id;
                    regs[1] = *non_blocking;
                    msg_size = 2;
                }
                Self::Bind(id, local_addr) => {
                    regs[0] = *id;
                    regs[1] = *local_addr;
                    extra_caps = 1;
                    msg_size = 2;
                }
                Self::Send(id, buf, buf_len) => {
                    regs[0] = *id;
                    regs[1] = *buf;
                    regs[2] = *buf_len;
                    extra_caps = 1;
                    msg_size = 3;
                }
                Self::Recv(id, buf, buf_len) => {
                    regs[0] = *id;
                    regs[1] = *buf;
                    regs[2] = *buf_len;
                    extra_caps = 1;
                    msg_size = 3;
                }
                Self::RecvTimeout(id, buf, buf_len, deadline_tick) => {
                    regs[0] = *id;
                    regs[1] = *buf;
                    regs[2] = *buf_len;
                    regs[3] = *deadline_tick;
                    extra_caps = 1;
                    msg_size = 4;
                }
                Self::Connect(id, remote_addr) => {
                    regs[0] = *id;
                    regs[1] = *remote_addr;
                    extra_caps = 1;
                    msg_size = 2;
                }
                Self::Listen(id) => {
                    regs[0] = *id;
                    msg_size = 1;
                }
                Self::Accept(id) => {
                    regs[0] = *id;
                    msg_size = 1;
                }
                Self::Shutdown(id) => {
                    regs[0] = *id;
                    msg_size = 1;
                }
                Self::Close(id) => {
                    regs[0] = *id;
                    msg_size = 1;
                }
            }
        });

        MessageInfo::new(self.to_label(), caps_unwrapped, extra_caps, msg_size)
    }
}

/// Page aligned with [GRANULE_SIZE]
#[repr(align(4096))]
pub struct AlignedPage(UnsafeCell<[u8; PAGE_SIZE]>);

impl AlignedPage {
    /// Create a new aligned page with [GRANULE_SIZE] of data
    pub const fn new() -> Self {
        Self(UnsafeCell::new([0; PAGE_SIZE]))
    }

    /// Get the ptr of the aligned page
    pub const fn ptr(&self) -> *mut u8 {
        self.0.get() as _
    }
}
