//! 定义一些进程默认的配置

use sel4::cap::Endpoint;

pub(crate) const CSPACE_DEPTH: usize = 64;
pub(crate) const LARGE_PAGE_SIZE: usize = 0x200000; // 2MB
pub(crate) const PAGE_SIZE: usize = 0x1000; // 4KB
pub(crate) const PPI_NUM: usize = 32;

/// 一些默认 slot 位置
/// 默认的父进程的 Endpoint
pub const DEFAULT_PARENT_EP: Endpoint = Endpoint::from_bits(18);

/// 默认的自身提供服务的 Endpoint
pub const DEFAULT_SERVE_EP: Endpoint = Endpoint::from_bits(19);

/// 默认存储 杂项 Untyped 的 SLOT
pub const DEFAULT_MISC_UNTYPED_SLOT: u64 = 23;

/// 默认存储 堆 Untyped 的 SLOT
pub const DEFAULT_MEM_UNTYPED_SLOT: u64 = 24;

/// 可分配的 slot 起始位置
pub const DEFAULT_CUSTOM_SLOT_START: u64 = 0x100;
