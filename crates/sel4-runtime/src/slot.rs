use core::ops::Range;

use slot_manager::{LeafSlot, SlotManager};
use spin::Mutex;

static SLOT_MANAGER: Mutex<SlotManager> = Mutex::new(SlotManager::empty());

/// 初始化 [SLOT_MANAGER]
pub fn init() {
    SLOT_MANAGER
        .lock()
        .init_empty_slots(Range::from(0x20..usize::MAX));
}

/// 申请一个 [LeafSlot]
#[inline]
pub fn alloc_slot() -> LeafSlot {
    SLOT_MANAGER.lock().alloc_slot()
}
