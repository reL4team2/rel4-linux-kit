//! 内存相关系统调用
//!
//!

use common::page::PhysPage;
use crate_consts::PAGE_SIZE;

use crate::{consts::task::DEF_HEAP_ADDR, task::Sel4Task, utils::obj::alloc_page};

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
    fd: usize,
    off: usize,
) -> SysResult {
    assert_eq!(fd, usize::MAX, "Map a file is not supperted now");
    assert_eq!(start % PAGE_SIZE, 0);
    debug!("MMAP @ {start:#x} {size:#x} {prot:#x} {flags:#x} {fd:#x} {off:#x}");
    warn!("mmap is just map a regular page RWX");
    if task.mem.lock().heap >= start + size && start >= DEF_HEAP_ADDR {
        warn!("Only supported the case that calling brk before");
        return Ok(start);
    }
    let start = task.find_free_area(start, size);

    for addr in (start..start + size).step_by(PAGE_SIZE) {
        task.map_page(addr, PhysPage::new(alloc_page()));
    }
    Ok(start)
}
