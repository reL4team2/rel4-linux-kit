//! 提供 seL4 进程内存相关能力封装
use crate::allocator::{MemCapAllocator, MemCapGlobalAllocator, VirtFrameAllocator};
use crate::capset::CapSet;
use crate::config::{LARGE_PAGE_SIZE, PAGE_SIZE};
use alloc::collections::BTreeMap;
use memory_addr::{AddrRange, MemoryAddr, PhysAddr, VirtAddr};
use sel4::CapTypeForFrameObject;
use sel4::cap::{Granule, Untyped, VSpace};
use sel4::{Cap, cap_type};
use spin::Mutex;

/// 进程内存管理器
pub struct MemoryManager {
    vspace: VSpace,
    obj_allocator: MemCapGlobalAllocator,
    frame_allocator: Mutex<VirtFrameAllocator>,

    // 为了进行虚实地址转换
    v2p_map: Mutex<BTreeMap<usize, AddrRange<usize>>>,
    p2v_map: Mutex<BTreeMap<usize, AddrRange<usize>>>,
}

impl MemoryManager {
    /// 创建一个新的内存管理器
    pub fn new(vspace: VSpace, untyped: Untyped, frame_start: usize, frame_size: usize) -> Self {
        let obj_allocator = MemCapGlobalAllocator::new(untyped);
        let v2p_map = Mutex::new(BTreeMap::new());
        let p2v_map = Mutex::new(BTreeMap::new());
        let frame_allocator = Mutex::new(VirtFrameAllocator::new(frame_start, frame_size));

        MemoryManager {
            vspace,
            obj_allocator,
            v2p_map,
            p2v_map,
            frame_allocator,
        }
    }

    /// 分配大页并进行 map
    pub fn large_page_map_alloc(&self, start: VirtAddr, size: usize) -> sel4::Result<PhysAddr> {
        if !start.is_aligned(LARGE_PAGE_SIZE) {
            return Err(sel4::Error::AlignmentError);
        }

        if size == 0 || (size % LARGE_PAGE_SIZE != 0) {
            return Err(sel4::Error::AlignmentError);
        }

        let caps = self
            .obj_allocator
            .alloc_large_pages(size / LARGE_PAGE_SIZE)?;
        let paddr = caps[0].frame_get_address()?;

        for (i, cap) in caps.iter().enumerate() {
            let vaddr_offset = start.add(i * LARGE_PAGE_SIZE);
            self.map_page::<cap_type::LargePage, MemCapGlobalAllocator>(
                vaddr_offset,
                cap,
                &self.obj_allocator,
            )?;
        }

        self.add_region(start.as_usize(), paddr, size);

        Ok(PhysAddr::from_usize(paddr))
    }

    /// 分配页并进行 map
    pub fn map_alloc(&self, start: VirtAddr, size: usize) -> sel4::Result<PhysAddr> {
        if !start.is_aligned(PAGE_SIZE) {
            return Err(sel4::Error::AlignmentError);
        }

        if (size % PAGE_SIZE != 0) || (size == 0) {
            return Err(sel4::Error::AlignmentError);
        }

        let caps = self.obj_allocator.alloc_pages(size / PAGE_SIZE)?;
        let paddr = caps[0].frame_get_address()?;

        for (i, cap) in caps.iter().enumerate() {
            let vaddr_offset = start.add(i * PAGE_SIZE);
            self.map_page::<cap_type::Granule, MemCapGlobalAllocator>(
                vaddr_offset,
                cap,
                &self.obj_allocator,
            )?;
        }

        self.add_region(start.as_usize(), paddr, size);

        Ok(PhysAddr::from_usize(paddr))
    }

    /// 虚拟地址转物理地址
    pub fn virt_to_phys(&self, vaddr: VirtAddr) -> sel4::Result<PhysAddr> {
        let lp_vstart = (vaddr.as_usize() / LARGE_PAGE_SIZE) * LARGE_PAGE_SIZE;
        let vstart = (vaddr.as_usize() / PAGE_SIZE) * PAGE_SIZE;

        if let Some(range) = self.v2p_map.lock().get(&lp_vstart) {
            let paddr = range.start + (vaddr.as_usize() - lp_vstart);
            if paddr < range.end {
                return Ok(PhysAddr::from_usize(paddr));
            }
        } else if let Some(range) = self.v2p_map.lock().get(&vstart) {
            let paddr = range.start + (vaddr.as_usize() - lp_vstart);
            if paddr < range.end {
                return Ok(PhysAddr::from_usize(paddr));
            }
        }

        return Err(sel4::Error::FailedLookup);
    }

    /// 物理地址转虚拟地址
    pub fn phys_to_virt(&self, paddr: PhysAddr) -> sel4::Result<VirtAddr> {
        let lp_pstart = (paddr.as_usize() / LARGE_PAGE_SIZE) * LARGE_PAGE_SIZE;
        let pstart = (paddr.as_usize() / PAGE_SIZE) * PAGE_SIZE;

        if let Some(range) = self.p2v_map.lock().get(&lp_pstart) {
            let vaddr = range.start + (paddr.as_usize() - lp_pstart);
            if vaddr < range.end {
                return Ok(VirtAddr::from_usize(vaddr));
            }
        } else if let Some(range) = self.p2v_map.lock().get(&pstart) {
            let vaddr = range.start + (paddr.as_usize() - pstart);
            if vaddr < range.end {
                return Ok(VirtAddr::from_usize(vaddr));
            }
        }

        return Err(sel4::Error::FailedLookup);
    }

    /// 分配一个 IPC Buffer 页并进行映射
    pub fn alloc_ipc_buffer(
        &self,
        allocator: Option<&mut CapSet>,
    ) -> sel4::Result<(VirtAddr, Granule)> {
        let ipc_vpn = self
            .frame_allocator
            .lock()
            .alloc()
            .ok_or(sel4::Error::NotEnoughMemory)?;

        let page_cap: Granule = match allocator {
            Some(alloc) => {
                let ipc_cap = alloc.alloc_page()?;
                self.map_page::<cap_type::Granule, CapSet>(
                    VirtAddr::from_usize(ipc_vpn * PAGE_SIZE),
                    &ipc_cap,
                    alloc,
                )?;
                ipc_cap
            }
            None => {
                let ipc_cap = self.obj_allocator.alloc_page()?;
                self.map_page::<cap_type::Granule, MemCapGlobalAllocator>(
                    VirtAddr::from_usize(ipc_vpn * PAGE_SIZE),
                    &ipc_cap,
                    &self.obj_allocator,
                )?;
                ipc_cap
            }
        };

        Ok((VirtAddr::from_usize(ipc_vpn * PAGE_SIZE), page_cap))
    }

    /// 释放一个 IPC Buffer 页
    pub fn dealloc_ipc_buffer(&self, vaddr: VirtAddr) {
        let vpn = vaddr.as_usize() / PAGE_SIZE;
        self.frame_allocator.lock().dealloc(vpn);
    }

    /// 添加一段虚实地址映射
    pub fn add_region(&self, vaddr: usize, paddr: usize, size: usize) {
        self.v2p_map
            .lock()
            .insert(vaddr, AddrRange::new(paddr, paddr + size));
        self.p2v_map
            .lock()
            .insert(paddr, AddrRange::new(vaddr, vaddr + size));
    }

    /// 映射一个页到虚拟地址
    fn map_page<T: CapTypeForFrameObject, C: MemCapAllocator>(
        &self,
        vaddr: VirtAddr,
        page: &Cap<T>,
        caps_alloc: &C,
    ) -> sel4::Result<()> {
        for _ in 0..sel4::vspace_levels::NUM_LEVELS {
            let res = page.frame_map(
                self.vspace,
                vaddr.as_usize() as _,
                sel4::CapRights::all(),
                sel4::VmAttributes::DEFAULT,
            );
            match res {
                Ok(_) => {
                    return Ok(());
                }
                Err(sel4::Error::FailedLookup) => {
                    let pt_cap = caps_alloc.alloc_pt()?;
                    pt_cap.pt_map(
                        self.vspace,
                        vaddr.as_usize() as _,
                        sel4::VmAttributes::DEFAULT,
                    )?;
                }
                _ => {
                    return res;
                }
            }
        }
        unreachable!("Failed to map large page at vaddr {:#x}", vaddr.as_usize());
    }
}
