//! 各种分配器，比如 slot index 分配器，内存分配器等等
use crate::config::PAGE_SIZE;
use alloc::vec;
use alloc::vec::Vec;
use common::ObjectAllocator;
use sel4::cap::Untyped;
use sel4::{Cap, cap_type};

/// Cap slot index 分配器
pub struct IndexAllocator {
    next: usize,
    max: usize,
    recycled: Vec<u64>,
}

impl IndexAllocator {
    /// 创建一个新的 IndexAllocator
    pub const fn new(next: usize, max: usize) -> Self {
        assert!(next <= max);
        Self {
            next,
            max,
            recycled: Vec::new(),
        }
    }

    /// 分配一个 slot index, 这个 index 是在当前 cnode 下的相对 index，不是在 cspace 中的绝对 index
    pub fn alloc(&mut self) -> Option<usize> {
        if self.next < self.max {
            let index = self.next;
            self.next += 1;
            Some(index)
        } else if let Some(index) = self.recycled.pop() {
            Some(index as usize)
        } else {
            None
        }
    }

    /// 回收一个 slot index
    pub fn recycle(&mut self, index: usize) {
        self.recycled.push(index as u64);
    }
}

/// 虚拟帧分配器，用于 IPC Buffer 分配
pub(crate) struct VirtFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>,
}

impl VirtFrameAllocator {
    pub(crate) const fn new(vstart: usize, size: usize) -> Self {
        VirtFrameAllocator {
            current: vstart / PAGE_SIZE,
            end: (vstart + size) / PAGE_SIZE,
            recycled: Vec::new(),
        }
    }

    pub(crate) fn alloc(&mut self) -> Option<usize> {
        if self.current == self.end {
            if let Some(vpn) = self.recycled.pop() {
                return Some(vpn);
            }
            return None;
        } else {
            let vpn = self.current;
            self.current += 1;
            return Some(vpn);
        }
    }

    #[allow(unused)]
    pub(crate) fn dealloc(&mut self, vpn: usize) {
        if vpn < self.current && !self.recycled.contains(&vpn) {
            self.recycled.push(vpn);
        }
    }
}

/// Untyped Cap 分配器
pub struct UntypedAllocator<'a> {
    recycled: Vec<Vec<sel4::cap::Untyped>>,
    obj_allocator: &'a ObjectAllocator,
    untyped_size: usize,
}

impl<'a> UntypedAllocator<'a> {
    /// 创建一个新的 UntypedAllocator
    /// 由于目前在 arceos 移植的设计是，每个 CPU 上有一个 seL4 任务，因此这儿需要传入 cpu_num
    /// 这个设计有点和 arceos 绑定了，后续可以考虑改进
    pub fn new(obj_allocator: &'a ObjectAllocator, untyped_size: usize, cpu_num: usize) -> Self {
        UntypedAllocator {
            recycled: vec![Vec::new(); cpu_num],
            obj_allocator,
            untyped_size,
        }
    }

    /// 在当前 cpu 的管理任务分配一个 untyped cap
    pub fn alloc(&mut self, cpu_id: usize) -> sel4::cap::Untyped {
        if let Some(cap) = self.recycled[cpu_id].pop() {
            cap
        } else {
            self.obj_allocator.alloc_untyped(self.untyped_size)
        }
    }

    /// 回收一个 untyped cap 到当前 cpu 的管理任务
    pub fn dealloc(&mut self, cap: sel4::cap::Untyped, cpu_id: usize) {
        self.recycled[cpu_id].push(cap);
    }
}

/// 内存相关能力分配器 trait
pub(crate) trait MemCapAllocator {
    /// 分配一个页表能力
    fn alloc_pt(&self) -> sel4::Result<Cap<cap_type::PT>>;
    /// 分配一个页能力
    fn alloc_page(&self) -> sel4::Result<Cap<cap_type::Granule>>;
    /// 分配多个页能力
    fn alloc_pages(&self, count: usize) -> sel4::Result<Vec<Cap<cap_type::Granule>>>;
    /// 分配一个大页能力
    fn alloc_large_page(&self) -> sel4::Result<Cap<cap_type::LargePage>>;
    /// 分配多个大页能力
    fn alloc_large_pages(&self, count: usize) -> sel4::Result<Vec<Cap<cap_type::LargePage>>>;
}

/// 全局内存能力分配器
pub struct MemCapGlobalAllocator {
    obj_allocator: ObjectAllocator,
}

impl MemCapGlobalAllocator {
    /// 创建一个新的全局内存能力分配器
    pub(crate) fn new(untyped: Untyped) -> Self {
        let obj_allocator = ObjectAllocator::empty();
        obj_allocator.init(untyped);
        MemCapGlobalAllocator { obj_allocator }
    }
}

impl MemCapAllocator for MemCapGlobalAllocator {
    fn alloc_pt(&self) -> sel4::Result<Cap<cap_type::PT>> {
        Ok(self.obj_allocator.alloc_pt())
    }

    fn alloc_page(&self) -> sel4::Result<Cap<cap_type::Granule>> {
        Ok(self.obj_allocator.alloc_page())
    }

    fn alloc_pages(&self, count: usize) -> sel4::Result<Vec<Cap<cap_type::Granule>>> {
        Ok(self.obj_allocator.alloc_pages(count))
    }

    fn alloc_large_page(&self) -> sel4::Result<Cap<cap_type::LargePage>> {
        Ok(self.obj_allocator.alloc_large_page())
    }

    fn alloc_large_pages(&self, count: usize) -> sel4::Result<Vec<Cap<cap_type::LargePage>>> {
        Ok(self.obj_allocator.alloc_large_pages(count))
    }
}
