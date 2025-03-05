#![no_std]

use sel4::cap::Endpoint;

/// The default notification for thread lock.
pub const DEFAULT_THREAD_NOTIFICATION: u64 = 17;
/// The default endpoint for thread lock.
pub const DEFAULT_PARENT_EP: Endpoint = Endpoint::from_bits(18);
/// The default endpoint for thread lock.
pub const DEFAULT_SERVE_EP: Endpoint = Endpoint::from_bits(19);
/// page_writer 用来占位的位置
/// TODO: 找一个更加合适的位置来放置，防止产生冲突
pub const DEFAULT_PAGE_PLACEHOLDER: u64 = 0;
/// The default slot to store custom cap.
pub const DEFAULT_CUSTOM_SLOT: u64 = 26;
/// The Default Index of the empty slot.
pub const DEFAULT_EMPTY_SLOT_INDEX: usize = 32;

// The radix bits of the cnode in the task.
pub const CNODE_RADIX_BITS: usize = 12;

pub const PAGE_SIZE_BITS: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SIZE_BITS;
pub const PAGE_MASK: usize = !0xfff;

/// Stack aligned with [STACK_ALIGN_SIZE] bytes
pub const STACK_ALIGN_SIZE: usize = 16;

/// The size of the granule.
pub const GRANULE_SIZE: usize = sel4::FrameObjectType::GRANULE.bytes();

/// The irq number of the serial device.
pub const SERIAL_DEVICE_IRQ: usize = 33;
pub const VIRTIO_NET_IRQ: usize = 0x2f + 0x20;

pub const DMA_ADDR_START: usize = 0x1_0000_3000;
