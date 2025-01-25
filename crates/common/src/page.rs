//! 提供页表操作的结构和方法抽象
//!
//! 这里提供一个占位页，然后将 sel4 的 Granule Capability 进行封装，作为 [PhysPage] 结构，提供一系列方法，用于操作物理页。
//! 需要使用的时候使用 [PhysPage::lock] 方法对 [PAGE_MAP_LOCK] 进行上锁，保证同时只有一个页表映射到空白占位页上，同获取一
//! 个 [PhysPageLocker] 对象，然后在这个对象上进行读写操作。在 [PhysPageLocker] 对象销毁的时候自动释放所占用的空白页。
#![deny(missing_docs)]
#![allow(static_mut_refs)]

use core::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

use alloc::slice;
use crate_consts::GRANULE_SIZE;
use sel4::{cap::Granule, init_thread::slot, CapRights, VmAttributes};

/// 空白页占位结构，保证数据 4k 对齐
#[repr(C, align(4096))]
pub struct FreePagePlaceHolder([u8; GRANULE_SIZE]);

impl FreePagePlaceHolder {
    /// 获取占位页的虚拟地址
    fn addr(&self) -> usize {
        self.0.as_ptr() as usize
    }
}

/// 空白页占位，需要写入读写页时将其映射到当前地址空间进行读写。
static mut FREE_PAGE_PLACEHOLDER: FreePagePlaceHolder = FreePagePlaceHolder([0; GRANULE_SIZE]);

/// 页映射锁，用于保护页表占位符，防止未释放时重复映射
static PAGE_MAP_LOCK: AtomicBool = AtomicBool::new(false);

/// 物理页表的抽象，提供一系列方法，用于操作物理页。
#[derive(Clone, Copy)]
pub struct PhysPage {
    cap: Granule,
}

impl PhysPage {
    /// 从 Capability 中创建一个物理页表
    pub const fn new(cap: Granule) -> Self {
        Self { cap }
    }

    /// 获取页表的物理地址
    #[inline]
    pub fn addr(&self) -> usize {
        self.cap
            .frame_get_address()
            .expect("can't get address of the physical page")
    }

    /// 获取页表的 Capability
    pub const fn cap(&self) -> Granule {
        self.cap
    }

    /// 锁定物理页表，返回一个物理页锁，可以在这个对象上进行读写
    pub fn lock(&self) -> PhysPageLocker {
        PAGE_MAP_LOCK.store(true, Ordering::SeqCst);
        let addr = unsafe { FREE_PAGE_PLACEHOLDER.addr() };
        self.cap
            .frame_map(
                slot::VSPACE.cap(),
                addr,
                CapRights::all(),
                VmAttributes::DEFAULT,
            )
            .unwrap();
        PhysPageLocker {
            cap: self.cap,
            data: unsafe { slice::from_raw_parts_mut(addr as _, GRANULE_SIZE) },
        }
    }
}

/// 物理页表锁，用于保护物理页表的读写
pub struct PhysPageLocker<'a> {
    cap: Granule,
    data: &'a mut [u8],
}

impl<'a> PhysPageLocker<'a> {
    /// 在 `offset` 处写入一个 usize 数据
    ///
    /// 需要保证 `offset` 为 `sizeof(usize)` 的整数倍
    #[inline]
    pub fn write_usize(&mut self, offset: usize, data: usize) {
        let len = core::mem::size_of::<usize>();
        self.data[offset..offset + len].copy_from_slice(&data.to_le_bytes());
    }

    /// 在 `offset` 出写入一个 bytes 序列
    ///
    /// 需要保证 offset + data.len() <= 4096 且 `offset` 为 `sizeof(usize)` 的整数倍
    #[inline]
    pub fn write_bytes(&mut self, offset: usize, data: &[u8]) {
        self.data[offset..offset + data.len()].copy_from_slice(data);
    }
}

impl<'a> Deref for PhysPageLocker<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'a> DerefMut for PhysPageLocker<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<'a> Drop for PhysPageLocker<'a> {
    fn drop(&mut self) {
        self.cap.frame_unmap().unwrap();
        PAGE_MAP_LOCK.store(false, Ordering::SeqCst);
    }
}

impl Debug for PhysPage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PhysPage")
            .field("cap", &(self.cap.frame_get_address()))
            .finish()
    }
}
