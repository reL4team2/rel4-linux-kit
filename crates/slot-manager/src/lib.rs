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
//! +-----------+-----------+---------+
//! |  63..24   |  23..12   |  11..0  |
//! +-----------+-----------+---------+
//! | Not Used  |  Level 1  | Level 0 |
//! +-----------+-----------+---------+
//!
//! 为什么叫 [LeafSlot] 也就是 [LeafSlot] 永远指向最后一级页表中的 Slot，`Level 1` 中指向的 Slot
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
    init_thread::{slot, Slot},
    AbsoluteCPtr, Cap, CapType,
};

/// 叶子 slot
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
}

/// Slot Manager
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
    pub fn new(empty_slots: Range<usize>) -> Self {
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
}