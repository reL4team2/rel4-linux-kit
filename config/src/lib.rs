//! rel4-linux-kit 配置 crate
//!
//! 这个 crate 中包含了整个系统的配置，需要在多个 crarte 中共享的配置信息。
#![no_std]
#![deny(missing_docs)]
#![deny(warnings)]

/// 服务任务在启动的时候使用的栈的地址。
pub const SERVICE_BOOT_STACK_TOP: usize = 0x1_0000_0000;

/// 服务任务在启动的时候使用的栈的大小
pub const SERVICE_BOOT_STACK_SIZE: usize = 0x1_0000;

/// 服务任务默认的堆大小
pub const SERVICE_HEAP_SIZE: usize = 0x20_0000;

/// VIRTIO_MMIO 使用的地址
pub const VIRTIO_MMIO_ADDR: usize = 0xa003e00;

/// PL011 设备使用过的地址
pub const PL011_ADDR: usize = 0x0900_0000;

/// 将要被映射的偏移地址，设备虚拟地址 = VIRT_ADDR + 设备物理地址
pub const VIRTIO_MMIO_VIRT_ADDR: usize = 0x1_2000_0000;

const VIRTIO_BLK_OFFSET: usize = 0x3e00;
const VIRTIO_NET_OFFSET: usize = 0x3c00;

/// VIRTIO 块设备使用的虚拟地址
pub const VIRTIO_MMIO_BLK_VIRT_ADDR: usize = VIRTIO_MMIO_VIRT_ADDR + VIRTIO_BLK_OFFSET;
/// VIRTIO 网络设备使用的虚拟地址
pub const VIRTIO_MMIO_NET_VIRT_ADDR: usize = VIRTIO_MMIO_VIRT_ADDR + VIRTIO_NET_OFFSET;

/// 串口的中断号
pub const SERIAL_DEVICE_IRQ: usize = 33;
/// VIRTIO 网络设备的中断号
pub const VIRTIO_NET_IRQ: usize = 0x2f + 0x20;

/// 默认的 DMA 分配开始的地址
pub const DMA_ADDR_START: usize = 0x1_0000_3000;

/// 默认 CSpace 一级占用的 bits
pub const CNODE_RADIX_BITS: usize = 12;

/// 默认的物理页大小
pub const PAGE_SIZE: usize = 0x1000;

/// 默认的页的 mask 位
pub const PAGE_MASK: usize = !0xfff;

/// 默认存储自定义 Capability 的 SLOT
pub const DEFAULT_CUSTOM_SLOT: u64 = 26;

/// 默认服务可分配的 SLOT 开始的地址
pub const DEFAULT_EMPTY_SLOT_INDEX: usize = 32;

/// 默认的栈对齐的大小
pub const STACK_ALIGN_SIZE: usize = 16;

/// 页共享使用的初始地址
pub const SHARE_PAGE_START: usize = 0x1_001F_0000;
