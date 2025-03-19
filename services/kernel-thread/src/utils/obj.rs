//! obj 管理模块，提供了对象的管理功能
use common::ObjectAllocator;
use config::DEFAULT_CUSTOM_SLOT;
use sel4::{
    Cap,
    cap::{CNode, Granule, Notification, PT, Tcb, VSpace},
};
use slot_manager::LeafSlot;
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

/// 申请一个内存页 [Granule]
#[inline]
pub fn alloc_page() -> Granule {
    OBJ_ALLOCATOR.lock().alloc_page()
}

/// 申请一个线程控制块 [Tcb]
#[inline]
pub fn alloc_tcb() -> Tcb {
    OBJ_ALLOCATOR.lock().alloc_tcb()
}

/// 申请一个页表 [PT]
#[inline]
pub fn alloc_pt() -> PT {
    OBJ_ALLOCATOR.lock().alloc_pt()
}

/// 申请一个虚拟地址空间 [VSpace]
#[inline]
pub fn alloc_vspace() -> VSpace {
    OBJ_ALLOCATOR.lock().alloc_vspace()
}

/// 申请一个 Capability 节点 [CNode]
#[inline]
pub fn alloc_cnode(size_bits: usize) -> CNode {
    OBJ_ALLOCATOR.lock().alloc_cnode(size_bits)
}

/// 初始化 [OBJ_ALLOCATOR]
pub fn init() {
    OBJ_ALLOCATOR
        .lock()
        .init(Cap::from_bits(DEFAULT_CUSTOM_SLOT as _));
}
