use memory_addr::{MemoryAddr, PageIter4K, VirtAddr};
use sel4::{debug_println, CapRights, CapRightsBuilder};
use syscalls::Errno;

use crate::{child_test::TASK_MAP, syscall::SysResult, OBJ_ALLOCATOR};

bitflags::bitflags! {
    #[derive(Debug, Clone)]
    /// permissions for sys_mmap
    ///
    /// See <https://github.com/bminor/glibc/blob/master/bits/mman.h>
    struct MmapProt: i32 {
        /// Page can be read.
        const PROT_READ = 1 << 0;
        /// Page can be written.
        const PROT_WRITE = 1 << 1;
        /// Page can be executed.
        const PROT_EXEC = 1 << 2;
    }
}

impl From<MmapProt> for CapRights {
    fn from(value: MmapProt) -> Self {
        let mut cap_builder = CapRightsBuilder::all();

        if !value.contains(MmapProt::PROT_READ) {
            cap_builder = cap_builder.read(false);
        }
        if !value.contains(MmapProt::PROT_WRITE) {
            cap_builder = cap_builder.write(false);
        }
        // if value.contains(MmapProt::PROT_EXEC) {
        // }
        cap_builder.build()
    }
}

bitflags::bitflags! {
    #[derive(Debug)]
    /// flags for sys_mmap
    ///
    /// See <https://github.com/bminor/glibc/blob/master/bits/mman.h>
    struct MmapFlags: i32 {
        /// Share changes
        const MAP_SHARED = 1 << 0;
        /// Changes private; copy pages on write.
        const MAP_PRIVATE = 1 << 1;
        /// Map address must be exactly as requested, no matter whether it is available.
        const MAP_FIXED = 1 << 4;
        /// Don't use a file.
        const MAP_ANONYMOUS = 1 << 5;
        /// Don't check for reservations.
        const MAP_NORESERVE = 1 << 14;
        /// Allocation is for a stack.
        const MAP_STACK = 0x20000;
    }
}

pub(crate) fn sys_mmap(
    badge: u64,
    addr: *mut usize,
    length: usize,
    prot: i32,
    flags: i32,
    _fd: i32,
    _offset: isize,
) -> SysResult {
    let mut task_map = TASK_MAP.lock();
    let task = task_map.get_mut(&badge).unwrap();
    let map_flags = MmapFlags::from_bits(flags).ok_or(Errno::EINVAL)?;
    let _permission_flags = MmapProt::from_bits(prot).ok_or(Errno::EINVAL)?;
    let start_addr = VirtAddr::from(if map_flags.contains(MmapFlags::MAP_FIXED) {
        addr as usize
    } else {
        // TODO: find a free area for the mapping
        task.find_free_area(addr as usize, length).unwrap()
    });
    let end = start_addr + length;
    for vaddr in PageIter4K::new(start_addr.align_down_4k(), end.align_up_4k())
        .expect("Failed to create page iterator")
    {
        if task.mapped_page.get(&(vaddr.as_usize())).is_some() {
            continue;
        }
        let page_cap = OBJ_ALLOCATOR
            .lock()
            .allocate_and_retyped_fixed_sized::<sel4::cap_type::Granule>();
        debug_println!("vaddr: {:?}, page_cap: {:?}", vaddr, page_cap);
        task.map_page(vaddr.as_usize(), page_cap);
    }

    Ok(start_addr.as_usize())
}

pub(crate) fn sys_unmap(badge: u64, addr: *mut usize, length: usize) -> SysResult {
    let mut task_map = TASK_MAP.lock();
    let task = task_map.get_mut(&badge).unwrap();
    let start_addr = VirtAddr::from(addr as usize);
    let end = start_addr + length;
    for vaddr in PageIter4K::new(start_addr.align_down_4k(), end.align_up_4k())
        .expect("Failed to create page iterator")
    {
        if let Some(page) = task.mapped_page.get(&(vaddr.as_usize())) {
            task.unmap_page(vaddr.as_usize(), *page);
        }
    }

    Ok(0)
}
