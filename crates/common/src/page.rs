//! 提供页表操作的结构和方法抽象
//!
//! 这里提供一个占位页，然后将 sel4 的 Granule Capability 进行封装，作为 [PhysPage] 结构，提供一系列方法，用于操作物理页。
//! 需要使用的时候使用 [PhysPage::lock] 方法对 [PAGE_MAP_LOCK] 进行上锁，保证同时只有一个页表映射到空白占位页上，同获取一
//! 个 [PhysPageLocker] 对象，然后在这个对象上进行读写操作。在 [PhysPageLocker] 对象销毁的时候自动释放所占用的空白页。
#![deny(missing_docs)]
#![allow(static_mut_refs)]

use core::{
    fmt::Debug,
    hint::spin_loop,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

use config::PAGE_SIZE;
use core::slice;
use sel4::{CapRights, VmAttributes, cap::Granule, init_thread::slot};
use slot_manager::LeafSlot;

use crate::consts::DEFAULT_PAGE_PLACEHOLDER;

/// 空白页占位结构，保证数据 4k 对齐
#[repr(C, align(4096))]
pub struct FreePagePlaceHolder([u8; PAGE_SIZE]);

impl FreePagePlaceHolder {
    /// 获取占位页的虚拟地址
    fn addr(&self) -> usize {
        self.0.as_ptr() as usize
    }
}

/// 空白页占位，需要写入读写页时将其映射到当前地址空间进行读写。
static mut FREE_PAGE_PLACEHOLDER: FreePagePlaceHolder = FreePagePlaceHolder([0; PAGE_SIZE]);

/// 页映射锁，用于保护页表占位符，防止未释放时重复映射
static PAGE_MAP_LOCK: AtomicBool = AtomicBool::new(false);

/// 物理页表的抽象，提供一系列方法，用于操作物理页。
#[derive(Clone)]
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
        while PAGE_MAP_LOCK.load(Ordering::SeqCst) {
            spin_loop();
        }
        PAGE_MAP_LOCK.store(true, Ordering::SeqCst);
        let slot = LeafSlot::new(DEFAULT_PAGE_PLACEHOLDER as _);
        slot.copy_from(&self.cap.into(), CapRights::all()).unwrap();
        let addr = unsafe { FREE_PAGE_PLACEHOLDER.addr() };
        let cap: Granule = slot.cap();
        cap.frame_map(
            slot::VSPACE.cap(),
            addr,
            CapRights::all(),
            VmAttributes::DEFAULT,
        )
        .unwrap();
        PhysPageLocker {
            cap,
            data: unsafe { slice::from_raw_parts_mut(addr as _, PAGE_SIZE) },
        }
    }
}

/// 物理页表锁，用于保护物理页表的读写
pub struct PhysPageLocker<'a> {
    cap: Granule,
    data: &'a mut [u8],
}

impl PhysPageLocker<'_> {
    /// 在 `offset` 处写入一个 usize 数据
    ///
    /// - `offset` 需要写入的位置，如果大于页大小，就会取余数
    /// - `data`   需要写入的数据
    ///
    /// 需要保证 `offset` 为 `sizeof(usize)` 的整数倍
    #[inline]
    pub fn write_usize(&mut self, mut offset: usize, data: usize) {
        offset %= PAGE_SIZE;
        let len = core::mem::size_of::<usize>();
        self.data[offset..offset + len].copy_from_slice(&data.to_le_bytes());
    }

    /// 在 `offset` 处写入一个 bytes 序列
    ///
    /// - `offset` 需要写入的位置，如果大于页大小，就会取余数
    /// - `data`   需要写入的数据
    ///
    /// 需要保证 offset + data.len() <= 4096 且 `offset` 为 `sizeof(usize)` 的整数倍
    #[inline]
    pub fn write_bytes(&mut self, mut offset: usize, data: &[u8]) {
        offset %= PAGE_SIZE;
        self.data[offset..offset + data.len()].copy_from_slice(data);
    }

    /// 在 `offset` 处写入一个 u8 数据
    ///
    /// - `offset` 需要写入的位置，如果大于页大小，就会取余数
    /// - `data`   需要写入的数据
    #[inline]
    pub fn write_u8(&mut self, mut offset: usize, data: u8) {
        offset %= PAGE_SIZE;
        self.data[offset] = data;
    }

    /// 在 `offset` 处读取一个 usize 数据
    ///
    /// - `offset` 需要读取的位置，如果大于页大小，就会取余数
    pub fn read_usize(&self, mut offset: usize) -> usize {
        offset %= PAGE_SIZE;
        unsafe { (self.data.as_ptr().add(offset) as *const usize).read() }
    }
}

impl Deref for PhysPageLocker<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl DerefMut for PhysPageLocker<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl Drop for PhysPageLocker<'_> {
    fn drop(&mut self) {
        self.cap.frame_unmap().unwrap();
        LeafSlot::from(self.cap).delete().unwrap();
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
