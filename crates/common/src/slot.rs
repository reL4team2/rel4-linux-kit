//! Slot管理模块
//!
//! 提供一个基础的 [SlotManager]，方便任务进行 [LeafSlot] 的申请和释放
use core::ops::Range;

use slot_manager::{LeafSlot, SlotManager};
use spin::Mutex;

static SLOT_MANAGER: Mutex<SlotManager> = Mutex::new(SlotManager::empty());

/// 初始化 [SLOT_MANAGER]
pub fn init(empty_slots: Range<usize>) {
    SLOT_MANAGER.lock().init_empty_slots(empty_slots);
}

/// 申请一个 [LeafSlot]
#[inline]
pub fn alloc_slot() -> LeafSlot {
    SLOT_MANAGER.lock().alloc_slot()
}

/// 申请多个 [LeafSlot]
///
/// - `num` 需要申请的 [LeafSlot] 数量
///
/// 说明： 申请的 [LeafSlot] 从返回的地方开始，调用 [LeafSlot::next_slot] 获取下一个
#[inline]
pub fn alloc_slots(num: usize) -> LeafSlot {
    SLOT_MANAGER.lock().alloc_slots(num)
}
