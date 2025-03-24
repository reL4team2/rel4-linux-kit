//! Slot管理模块
//!
//! 提供一个基础的 [SlotManager]，方便任务进行 [LeafSlot] 的申请和释放
use core::ops::Range;

use slot_manager::{LeafSlot, SlotManager};
use spin::{Mutex, once::Once};

static SLOT_MANAGER: Mutex<SlotManager> = Mutex::new(SlotManager::empty());
static SLOT_EDGE_HANDLER: Once<fn() -> LeafSlot> = Once::new();

/// 初始化 [SLOT_MANAGER]
///
/// - `empty_slots`  空白的 slot 范围
/// - `handler`      当申请的 slot 在一级 CSpace 的边缘时的处理函数
pub fn init(empty_slots: Range<usize>, handler: Option<fn() -> LeafSlot>) {
    SLOT_MANAGER.lock().init_empty_slots(empty_slots);
    if let Some(handler) = handler {
        SLOT_EDGE_HANDLER.call_once(|| handler);
    }
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
