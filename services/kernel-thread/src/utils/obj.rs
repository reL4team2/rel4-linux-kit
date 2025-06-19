//! obj 管理模块，提供了对象的管理功能
use alloc::vec::Vec;
use common::{ObjectAllocator, config::DEFAULT_CUSTOM_SLOT};
use sel4::{
    Cap,
    cap::{Notification, PT},
    cap_type,
};
use sel4_kit::slot_manager::LeafSlot;
use spin::Mutex;

/// The object allocator for the kernel thread.
pub(crate) static OBJ_ALLOCATOR: Mutex<ObjectAllocator> = Mutex::new(ObjectAllocator::empty());

/// 申请一个空的 [LeafSlot]
#[inline]
pub fn alloc_slot() -> LeafSlot {
    OBJ_ALLOCATOR.lock().allocate_slot()
}

/// 申请一个 [Notification]
#[inline]
pub fn alloc_notification() -> Notification {
    OBJ_ALLOCATOR.lock().alloc_notification()
}

/// 申请一个页表 [PT]
#[inline]
pub fn alloc_pt() -> PT {
    OBJ_ALLOCATOR.lock().alloc_pt()
}

/// 初始化 [OBJ_ALLOCATOR]
pub fn init() {
    OBJ_ALLOCATOR
        .lock()
        .init(Cap::from_bits(DEFAULT_CUSTOM_SLOT as _));
}

const ALLOC_SIZE_BITS: usize = 22; // 4MB

static RECYCLED_UNTYPED: Mutex<Vec<Cap<cap_type::Untyped>>> = Mutex::new(Vec::new());

/// 申请一个未类型化的单元，每一个单元会作为可重新分配的单元使用
pub fn alloc_untyped_unit() -> (Cap<cap_type::Untyped>, usize) {
    let cap = RECYCLED_UNTYPED
        .lock()
        .pop()
        .unwrap_or_else(|| OBJ_ALLOCATOR.lock().alloc_untyped(ALLOC_SIZE_BITS));
    (cap, 1 << ALLOC_SIZE_BITS)
}

/// 回收一个未类型化的单元
pub fn recycle_untyped_unit(cap: Cap<cap_type::Untyped>) {
    LeafSlot::from_cap(cap).revoke().unwrap();
    RECYCLED_UNTYPED.lock().push(cap);
}
