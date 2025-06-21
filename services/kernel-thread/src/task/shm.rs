//! 共享内存模块
//!
//!
use alloc::{collections::btree_map::BTreeMap, sync::Arc, vec::Vec};
use common::{mem::CapMemSet, slot::recycle_slot};
use sel4::cap::Granule;
use sel4_kit::slot_manager::LeafSlot;
use spin::Mutex;

use crate::utils::obj::recycle_untyped_unit;

/// 共享内存的全局静态变量
pub static SHARED_MEMORY: Mutex<BTreeMap<usize, Arc<SharedMemory>>> = Mutex::new(BTreeMap::new());

/// 共享内存结构体
pub struct SharedMemory {
    /// 内存能力集
    pub capset: Mutex<CapMemSet>,
    /// 物理页跟踪器
    pub trackers: Vec<Granule>,
    /// 是否已删除
    pub deleted: Mutex<bool>,
}

impl SharedMemory {
    /// 创建一个新的共享内存实例
    ///
    /// # 参数
    /// - `capset`: 内存能力集
    /// - `trackers`: 物理页跟踪器
    pub const fn new(capset: Mutex<CapMemSet>, trackers: Vec<Granule>) -> Self {
        Self {
            capset,
            trackers,
            deleted: Mutex::new(false),
        }
    }
}

impl Drop for SharedMemory {
    fn drop(&mut self) {
        self.trackers.iter().for_each(|cap| {
            let slot = LeafSlot::from_cap(*cap);
            slot.revoke().unwrap();
            slot.delete().unwrap();
            recycle_slot(slot);
        });
        self.capset
            .lock()
            .untyped_list()
            .iter()
            .for_each(|(ut, _)| {
                recycle_untyped_unit(*ut);
            });
    }
}

/// 映射的共享内存结构体
#[derive(Clone)]
pub struct MapedSharedMemory {
    /// 共享内存的键
    pub key: usize,
    /// 共享内存的引用计数
    pub mem: Arc<SharedMemory>,
    /// 映射的起始地址
    pub start: usize,
    /// 映射的大小
    pub size: usize,
}

impl MapedSharedMemory {
    /// 检测虚拟地址是否在映射范围内
    pub fn contains(&self, vaddr: usize) -> bool {
        vaddr >= self.start && vaddr < self.start + self.size
    }
}

impl Drop for MapedSharedMemory {
    fn drop(&mut self) {
        // 当前任务持有一个，[SHARED_MEMORY] 持有一个
        if Arc::strong_count(&self.mem) == 2 && *self.mem.deleted.lock() {
            SHARED_MEMORY.lock().remove(&self.key);
        }
    }
}
