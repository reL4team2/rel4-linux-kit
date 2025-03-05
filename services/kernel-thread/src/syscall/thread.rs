//! 线程相关系统调用
//!
//!

use crate::task::Sel4Task;

use super::SysResult;

/// 获取进程 id
pub fn sys_getpid(task: &Sel4Task) -> SysResult {
    Ok(task.pid)
}

/// 获取父进程 id
pub fn sys_getppid(task: &Sel4Task) -> SysResult {
    Ok(task.ppid)
}

#[inline]
pub(super) fn sys_set_tid_addr(task: &mut Sel4Task, addr: usize) -> SysResult {
    task.clear_child_tid = Some(addr);
    Ok(task.id)
}
