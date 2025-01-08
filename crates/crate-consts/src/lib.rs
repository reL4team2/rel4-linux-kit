#![no_std]

use sel4::cap::Endpoint;

/// The default notification for thread lock.
pub const DEFAULT_THREAD_NOTIFICATION: u64 = 17;
/// The default endpoint for thread lock.
pub const DEFAULT_PARENT_EP: u64 = 18;
/// The default endpoint for thread lock.
pub const DEFAULT_SERVE_EP: u64 = 19;
/// The default endpoint for thread IRQ.
pub const DEFAULT_THREAD_IRQ_EP: u64 = 20;
/// The default slot to store custom cap.
pub const DEFAULT_CUSTOM_SLOT: u64 = 26;
/// The Default Index of the empty slot.
pub const DEFAULT_EMPTY_SLOT_INDEX: usize = 32;
/// The default slot to store thread recv cap.
pub const DEFAULT_THREAD_RECV_SLOT: u64 = (KERNEL_THREAD_SLOT_NUMS - 1) as _;

// Init End point, used in tasks.
pub const INIT_EP: Endpoint = Endpoint::from_bits(DEFAULT_PARENT_EP);

// CNode Bits
pub const DEFAULT_CNODE_BITS: u64 = 12;
pub const DEFAULT_CNODE_SLOT_NUMS: usize = 1 << DEFAULT_CNODE_BITS;
pub const KERNEL_THREAD_SLOT_NUMS: usize = 1 << 10;

// The radix bits of the cnode in the task.
pub const CNODE_RADIX_BITS: usize = 12;

pub const PAGE_SIZE_BITS: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SIZE_BITS;

/// Stack aligned with [STACK_ALIGN_SIZE] bytes
pub const STACK_ALIGN_SIZE: usize = 16;

/// The size of the granule.
pub const GRANULE_SIZE: usize = sel4::FrameObjectType::GRANULE.bytes();

/// The irq number of the serial device.
pub const SERIAL_DEVICE_IRQ: usize = 33;
pub const VIRTIO_NET_IRQ: usize = 0x2f + 0x20;

pub const DMA_ADDR_START: usize = 0x1_0000_3000;
