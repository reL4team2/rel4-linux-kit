//! 系统调用文件模块
//!
//!

use sel4_root_task::debug_print;
use syscalls::Errno;
use zerocopy::{FromBytes, Immutable};

use crate::task::Sel4Task;

use super::SysResult;

pub(super) fn sys_write(task: &Sel4Task, fd: usize, buf: *const u8, len: usize) -> SysResult {
    if fd != 1 && fd != 2 {
        return Err(Errno::EPERM);
    }
    let buf = task.read_bytes(buf as _, len).unwrap();

    let output = core::str::from_utf8(&buf).unwrap();
    debug_print!("{output}");
    Ok(len)
}

#[repr(C)]
#[derive(Clone, FromBytes, Immutable)]
pub(super) struct IoVec {
    pub base: usize,
    pub len: usize,
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
