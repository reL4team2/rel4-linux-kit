//! slot-manager crate
//!
//! 这个 crate 中提供两个 struct [LeafSlot] 和 [SlotManager]
//!
//! ## [LeafSlot]
//!
//! [LeafSlot] 对 [sel4] 提供的 slot 进行抽象，可以对 slot 进行管理
//! 和操作，或者从特定的 Slot 中创建一个新的 [LeafSlot]。
//!
//! [LeafSlot] 采用类似页表的设计，每个 CNode 有 4096 个 Slot (size_bits 为 12)，
//! 总体采用多级设计，目前准备采用 2 级设计，一共可以容纳 2^24 个 Slot，可以满足大部分的需求。
//! 如果只容纳物理页，那么最大可以使用 2^24 个物理页，也就是 64G 的物理内存
//!
//! ```plain
//! +-----------+-----------+---------+
//! |  63..24   |  23..12   |  11..0  |
//! +-----------+-----------+---------+
//! | Not Used  |  Level 1  | Level 0 |
//! +-----------+-----------+---------+
//! ```
//! 为什么叫 [LeafSlot] 是因为 [LeafSlot] 永远指向最后一级页表中的 Slot，`Level 1` 中指向的 Slot
//! 也叫 `NonLeafSlot`，中永远只包含 `CNode`。
//!
//! > 注意：root-task 启动时默认仅一级，只能容纳 4096 个 slot, 且有一些 slot 已经安排了内容，
//! > 如果需要在 root-task 中使用，需要重建 CSpace。这里暂时不提供重建的函数。
//!
//! ## [SlotManager]
//!
//! [SlotManager] 通过指定空 Slot 的范围创建，对空 slot 进行管理，可以申请新的 slot 位置。
//! 申请后范围的结构为 [LeafSlot]
//!

#![no_std]
#![deny(warnings)]
#![deny(missing_docs)]

use core::ops::Range;

use sel4::{
    AbsoluteCPtr, Cap, CapRights, CapType,
    init_thread::{Slot, slot},
};

/// 叶子 slot
///
/// CSpace 是一个层级结构，也可以理解为一棵树
/// 叶子 slot 就是在最边缘的位置，深度永远为 64(最大)
/// 这里设计每一级的深度为 12
/// 所以叶子 slot 的父亲深度为 52
#[derive(Debug, Clone, Copy)]
pub struct LeafSlot {
    idx: usize,
}

impl LeafSlot {
    /// 创建新的 Slot
    pub const fn new(idx: usize) -> Self {
        Self { idx }
    }

    /// 从 [Slot] 中创建一个 [LeafSlot]
    pub const fn from_slot<T: CapType>(slot: Slot<T>) -> Self {
        Self {
            idx: slot.cptr_bits() as _,
        }
    }

    /// 从 [Cap] 中创建一个 [LeafSlot]
    pub const fn from_cap<T: CapType>(cap: Cap<T>) -> Self {
        Self {
            idx: cap.bits() as _,
        }
    }

    /// 获取当前节点的绝对位置
    pub fn abs_cptr(&self) -> AbsoluteCPtr {
        slot::CNODE
            .cap()
            .absolute_cptr_from_bits_with_depth(self.idx as _, 64)
    }

    /// 获取父 CNode 节点的绝对位置
    pub fn cnode_abs_cptr(&self) -> AbsoluteCPtr {
        slot::CNODE
            .cap()
            .absolute_cptr_from_bits_with_depth(self.cnode_idx() as _, 52)
    }

    /// 获取父 CNode 节点的索引
    pub const fn cnode_idx(&self) -> usize {
        self.idx >> 12
    }

    /// 获取在 CNode 中的相对位置
    pub const fn offset_of_cnode(&self) -> usize {
        self.idx & 0xfff
    }

    /// 获取原始值
    pub const fn raw(&self) -> usize {
        self.idx
    }

    /// 获取 [LeafSlot] 指向的 slot 中的 Cap
    ///
    /// 如果这个 Slot 为空也可以获取，但是调用时会出现错误
    pub const fn cap<T: CapType>(&self) -> Cap<T> {
        Slot::from_index(self.idx).cap()
    }

    /// 获取这个位置后面的一个 [LeafSlot]
    ///
    /// slot 的数量不应该大于 CSpace 构建的最大数量
    pub const fn next_slot(&self) -> LeafSlot {
        assert!(self.idx < usize::MAX);
        LeafSlot::new(self.idx)
    }

    /// 获取这个位置后面第 n 个位置的 [LeafSlot]
    ///
    /// slot 的数量不应该大于 CSpace 构建的最大数量
    pub const fn next_nth_slot(&self, n: usize) -> LeafSlot {
        assert!(self.idx <= usize::MAX - n);
        LeafSlot::new(self.idx + n)
    }

    /// 从 `other_slot` 中复制一个 Capability
    ///
    /// 如果发生错误将返回 [sel4::Error]
    #[inline]
    pub fn copy_from(&self, other: &LeafSlot, rights: CapRights) -> Result<(), sel4::Error> {
        self.abs_cptr().copy(&other.abs_cptr(), rights)
    }

    /// 删除当前 [LeafSlot] 中的 Capability
    ///
    /// 如果需要删除 [sel4::cap::CNode] 下面的所有 Capability，需要先使用 [Self::revoke] 删除
    /// 派生出的 Capability，然后再调用 [AbsoluteCPtr::delete] 删除 slot 中的 Capability
    #[inline]
    pub fn delete(&self) -> Result<(), sel4::Error> {
        self.abs_cptr().delete()
    }

    /// 删除当前 [LeafSlot] 中派生出的 Capability
    ///
    /// 不会删除自身，需要调用 [Self::delete] 删除自身
    #[inline]
    pub fn revoke(&self) -> Result<(), sel4::Error> {
        self.abs_cptr().revoke()
    }

    /// 复制 badge 并设置权限
    ///
    /// - `dst`   复制后的 Cap 放在哪个 [LeafSlot]
    /// - `cr`    复制后的 Cap 的权限
    /// - `badge` 需要设置的 badge
    #[inline]
    pub fn mint_to(&self, dst: LeafSlot, cr: CapRights, badge: usize) -> Result<(), sel4::Error> {
        dst.abs_cptr().mint(&self.abs_cptr(), cr, badge as _)
    }

    /// 将 Capability 移动到指定的 [LeafSlot]
    ///
    /// - `dst`  需要移动到的 [LeafSlot]
    #[inline]
    pub fn move_to(&self, dst: Self) -> Result<(), sel4::Error> {
        dst.abs_cptr().move_(&self.abs_cptr())
    }

    /// 保存回复 Capability
    pub fn save_caller(&self) -> Result<(), sel4::Error> {
        self.abs_cptr().save_caller()
    }
}

/// [Cap] 可以快速转换为 [LeafSlot]
impl<T: CapType> From<Cap<T>> for LeafSlot {
    fn from(value: Cap<T>) -> Self {
        Self::from_cap(value)
    }
}

/// [LeafSlot] 快速转换为 [Cap]
impl<T: CapType> From<LeafSlot> for Cap<T> {
    fn from(value: LeafSlot) -> Self {
        value.cap()
    }
}

/// Slot 管理器
///
/// Slot 管理器，可以管理和申请特定的 Slot
#[derive(Debug)]
pub struct SlotManager {
    empty_slots: Range<usize>,
}

impl SlotManager {
    /// 创建一个空 Slot Manager，默认为没有空 Slot
    ///
    /// 随后在 [SlotManager::init_empty_slots] 中更新
    pub const fn empty() -> SlotManager {
        Self { empty_slots: 0..0 }
    }

    /// 从 empty slots 创建 Slot Manager
    pub const fn new(empty_slots: Range<usize>) -> Self {
        Self { empty_slots }
    }

    /// 初始化空的 slot 范围
    ///
    /// 一般配合 [SlotManager::empty] 使用
    pub const fn init_empty_slots(&mut self, new_slots: Range<usize>) {
        self.empty_slots = new_slots;
    }

    /// 申请一个新的空 Slot
    #[inline]
    pub fn alloc_slot(&mut self) -> LeafSlot {
        let idx = self.empty_slots.next().unwrap();
        LeafSlot::new(idx)
    }

    /// 申请多个 slot
    ///
    /// 返回的是开始位置的 LeafSlot
    #[inline]
    pub fn alloc_slots(&mut self, num: usize) -> LeafSlot {
        let idx = self.empty_slots.next().unwrap();
        self.empty_slots.start += num - 1;
        LeafSlot::new(idx)
    }
}
