//! 内存相关系统调用
//!
//!

use common::page::PhysPage;
use config::PAGE_SIZE;
use syscalls::Errno;

use crate::{
    consts::task::DEF_HEAP_ADDR, syscall::types::mm::MapFlags, task::Sel4Task,
    utils::obj::alloc_page,
};

use super::SysResult;

#[inline]
pub(super) fn sys_brk(task: &mut Sel4Task, heap: usize) -> SysResult {
    debug!("BRK @ heap: {heap:#x}");
    Ok(task.brk(heap))
}

#[inline]
pub(super) fn sys_mmap(
    task: &mut Sel4Task,
    start: usize,
    size: usize,
    prot: usize,
    flags: usize,
    fd: isize,
    off: usize,
) -> SysResult {
    let flags = MapFlags::from_bits_truncate(flags as _);
    if flags.contains(MapFlags::MAP_SHARED) {
        log::warn!("mmap share is not supported now!");
    }
    assert_eq!(start % PAGE_SIZE, 0);
    debug!("MMAP @ {start:#x} {size:#x} {prot:#x} {flags:#x} {fd:#x} {off:#x}");
    warn!("mmap is just map a regular page RWX");
    if task.mem.lock().heap >= start + size && start >= DEF_HEAP_ADDR {
        warn!("Only supported the case that calling brk before");
        return Ok(start);
    }
    let start = task.find_free_area(start, size);

    if fd > 0 {
        let fd = task
            .file
            .file_ds
            .lock()
            .get(fd as _)
            .ok_or(Errno::EINVAL)?
            .clone();
        let origin_off = fd.lock().seek(0, 1);
        fd.lock().seek(0, 0);
        let data = fd.lock().read_all().unwrap();
        fd.lock().seek(origin_off as _, 0);
        for addr in (start..start + size).step_by(PAGE_SIZE) {
            task.map_page(addr, PhysPage::new(alloc_page()));
        }
        task.write_bytes(start, &data);
    } else {
        for addr in (start..start + size).step_by(PAGE_SIZE) {
            task.map_page(addr, PhysPage::new(alloc_page()));
        }
    }
    Ok(start)
}

pub(super) fn sys_munmap(task: &Sel4Task, start: usize, len: usize) -> SysResult {
    debug!("sys_munmap @ start: {:#x}, len: {:#x}", start, len);
    task.mem.lock().mapped_page.retain(|vaddr, x| {
        if (start..start + len).contains(vaddr) {
            x.cap().frame_unmap().unwrap();
            false
        } else {
            true
        }
    });
    Ok(0)
}
