//! 线程相关系统调用
//!
//!

use crate::task::Sel4Task;

use super::SysResult;

/// 获取进程 id
#[inline]
pub fn sys_getpid(task: &Sel4Task) -> SysResult {
    Ok(task.pid)
}

/// 获取父进程 id
#[inline]
pub fn sys_getppid(task: &Sel4Task) -> SysResult {
    Ok(task.ppid)
}

#[inline]
pub(super) fn sys_set_tid_addr(task: &mut Sel4Task, addr: usize) -> SysResult {
    task.clear_child_tid = Some(addr);
    Ok(task.id)
}

#[inline]
pub(super) fn sys_exit(task: &mut Sel4Task, exit_code: i32) -> SysResult {
    debug!("sys_exit @ exit_code: {} ", exit_code);
    task.exit = Some(exit_code);
    Ok(0)
}

#[inline]
pub(super) fn sys_clone(
    task: &Sel4Task,
    flags: usize,     // 复制 标志位
    stack: usize,     // 指定新的栈，可以为 0, 0 不处理
    ptid: *const u32, // 父线程 id
    tls: usize,       // TLS线程本地存储描述符
    ctid: *const u32, // 子线程 id
) -> SysResult {
    log::debug!(
        "flags: {:#x} stack: {:#x}, ptid: {:p}  tls: {:#x}, ctid: {:#p}",
        flags,
        stack,
        ptid,
        tls,
        ctid
    );

    todo!()
}
