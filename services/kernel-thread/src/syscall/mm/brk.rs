use common::{USPACE_HEAP_BASE, USPACE_HEAP_SIZE};
use syscalls::Errno;

use crate::{child_test::TASK_MAP, syscall::SysResult};

pub(crate) fn sys_brk(badge: u64, addr: *mut u8) -> SysResult {
    let mut task_map = TASK_MAP.lock();
    let task = task_map.get_mut(&badge).unwrap();
    let addr = addr as usize;
    if addr < USPACE_HEAP_BASE || addr > USPACE_HEAP_BASE + USPACE_HEAP_SIZE {
        return Err(Errno::ENOMEM);
    }
    if addr > task.heap {
        task.brk(addr);
    }
    task.heap = addr;
    Ok(0)
}
