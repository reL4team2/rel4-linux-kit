//! Slot管理模块
//!
//! 提供一个基础的 [SlotManager]，方便任务进行 [LeafSlot] 的申请和释放
use core::ops::Range;

use sel4::{init_thread, with_ipc_buffer_mut};
use sel4_kit::slot_manager::{LeafSlot, SlotManager};
use spin::{Mutex, once::Once};

static SLOT_MANAGER: Mutex<SlotManager> = Mutex::new(SlotManager::empty());
static SLOT_EDGE_HANDLER: Once<fn(LeafSlot)> = Once::new();

/// 初始化 [SLOT_MANAGER]
///
/// - `empty_slots`  空白的 slot 范围
pub fn init(empty_slots: Range<usize>) {
    SLOT_MANAGER.lock().init_empty_slots(empty_slots);
}

/// 设置 slot 在边缘时的处理函数
///
/// - `handler`      当申请的 slot 在一级 CSpace 的边缘时的处理函数
pub fn init_slot_edge_handler(handler: fn(LeafSlot)) {
    SLOT_EDGE_HANDLER.call_once(|| handler);
}

/// 申请一个 [LeafSlot]
#[inline]
pub fn alloc_slot() -> LeafSlot {
    let mut slot_manager = SLOT_MANAGER.lock();
    if slot_manager.available() == 0 {
        SLOT_EDGE_HANDLER.get().unwrap()(LeafSlot::new(slot_manager.next_range_start()));
        slot_manager.extend(0x1000);
    }
    slot_manager.alloc_slot()
}

/// 申请多个 [LeafSlot]
///
/// - `num` 需要申请的 [LeafSlot] 数量
///
/// 说明： 申请的 [LeafSlot] 从返回的地方开始，调用 [LeafSlot::next_slot] 获取下一个
#[inline]
pub fn alloc_slots(num: usize) -> LeafSlot {
    SLOT_MANAGER.lock().alloc_slots(num).next().unwrap()
}

/// 释放一个 [LeafSlot]
///
/// # 参数
/// - `slot` 需要释放的 [LeafSlot]
#[cfg(feature = "alloc")]
pub fn recycle_slot(slot: LeafSlot) {
    SLOT_MANAGER.lock().recycle_slot(slot);
}

/// 初始化接收 Slot
pub fn init_recv_slot() {
    with_ipc_buffer_mut(|ipc_buffer| {
        ipc_buffer.set_recv_slot(
            &init_thread::slot::CNODE
                .cap()
                .absolute_cptr_from_bits_with_depth(0, 64),
        );
    })
}
