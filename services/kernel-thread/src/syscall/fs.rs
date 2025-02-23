//! 系统调用文件模块
//!
//!

use syscalls::Errno;
use zerocopy::FromBytes;

use crate::task::Sel4Task;

use super::{types::IoVec, SysResult};

pub(super) fn sys_write(task: &Sel4Task, fd: usize, buf: *const u8, len: usize) -> SysResult {
    if fd != 1 && fd != 2 {
        return Err(Errno::EPERM);
    }
    let buf = task.read_bytes(buf as _, len).unwrap();

    for b in buf.iter() {
        sel4::debug_put_char(*b);
    }
    Ok(len)
}

pub(super) fn sys_writev(task: &Sel4Task, fd: usize, iov: *const IoVec, iocnt: usize) -> SysResult {
    let mut wsize = 0;
    let iovec_bytes = task
        .read_bytes(iov as _, size_of::<IoVec>() * iocnt)
        .unwrap();

    let iovec = <[IoVec]>::ref_from_bytes_with_elems(&iovec_bytes, iocnt).unwrap();
    for item in iovec.iter() {
        sys_write(task, fd, item.base as _, item.len)?;
        wsize += item.len;
    }

    Ok(wsize)
}

pub(super) fn sys_getcwd(task: &Sel4Task, buf: *mut u8, _size: usize) -> SysResult {
    log::warn!("get cwd is a simple implement, always return /");
    task.write_bytes(buf as _, b"/");

    Ok(buf as _)
}
