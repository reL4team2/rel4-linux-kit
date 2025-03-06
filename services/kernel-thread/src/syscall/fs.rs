//! 系统调用文件模块
//!
//!

use alloc::{string::String, sync::Arc};
use common::services::fs::Stat;
use spin::mutex::Mutex;
use syscalls::Errno;
use zerocopy::{FromBytes, IntoBytes};

use crate::{fs::file::File, task::Sel4Task};

use super::{SysResult, types::IoVec};

const FD_CUR_DIR: isize = -100;

pub(super) fn sys_chdir(task: &mut Sel4Task, buf: *const u8) -> SysResult {
    let path_bytes = task.read_cstr(buf as _).unwrap();
    let path = String::from_utf8(path_bytes).unwrap();

    if path.starts_with('.') {
        panic!("relative path is not supported now!")
    } else if path.starts_with('/') {
        task.file.work_dir = path;
    } else {
        task.file.work_dir += &path;
        task.file.work_dir += "/";
    }

    Ok(0)
}

pub(super) fn sys_close(task: &Sel4Task, fd: usize) -> SysResult {
    task.file.file_ds.lock().remove(fd).ok_or(Errno::EBADF)?;
    Ok(0)
}

pub(super) fn sys_dup(task: &Sel4Task, fd: usize) -> SysResult {
    let mut file_table = task.file.file_ds.lock();
    let old_fd = file_table.get(fd).ok_or(Errno::EBADF)?.clone();
    file_table.add(old_fd).map_err(|_| Errno::EBADFD)
}

pub(super) fn sys_dup3(task: &Sel4Task, fd: usize, fd_dst: usize) -> SysResult {
    let mut file_table = task.file.file_ds.lock();
    let old_fd = file_table.get(fd).ok_or(Errno::EBADF)?.clone();
    file_table.add_at(fd_dst, old_fd).map_err(|_| Errno::EBADF)
}

pub(super) fn sys_fstat(task: &Sel4Task, fd: usize, stat_ptr: *mut Stat) -> SysResult {
    let file_table = task.file.file_ds.lock();
    let file = file_table.get(fd).ok_or(Errno::EBADF)?.clone();

    let stat = file.lock().stat()?;
    task.write_bytes(stat_ptr as _, stat.as_bytes());
    Ok(0)
}

pub(super) fn sys_getdents64(
    task: &Sel4Task,
    fd: usize,
    buf_ptr: *const u8,
    len: usize,
) -> SysResult {
    debug!(
        "[task {}] sys_getdents64 @ fd: {}, buf_ptr: {:p}, len: {}",
        task.id, fd, buf_ptr, len
    );
    let file_table = task.file.file_ds.lock();
    let file = file_table.get(fd).ok_or(Errno::EBADF)?.clone();
    todo!("sys_getdents64")
}

pub(super) fn sys_read(task: &Sel4Task, fd: usize, bufp: *const u8, count: usize) -> SysResult {
    if count == 0 {
        return Err(Errno::EINVAL);
    }
    let mut file_table = task.file.file_ds.lock();
    let file = file_table.get_mut(fd).ok_or(Errno::EBADF)?;
    let mut buffer = vec![0u8; count];
    let rlen = file.lock().read(&mut buffer)?;
    task.write_bytes(bufp as _, &buffer);
    Ok(rlen)
}

pub(super) fn sys_unlinkat(task: &Sel4Task, fd: isize, path: *const u8, _flags: u64) -> SysResult {
    let path_bytes = task.read_cstr(path as _).unwrap();
    let mut path = String::from_utf8(path_bytes).unwrap();

    let dir_path = if fd == FD_CUR_DIR {
        task.file.work_dir.clone()
    } else if fd > 0 {
        let dir = task
            .file
            .file_ds
            .lock()
            .get(fd as _)
            .ok_or(Errno::EBADF)?
            .clone();

        dir.lock().path() + "/"
    } else {
        panic!("not supported")
    };

    if let Some(strip_path) = path.strip_prefix("./") {
        path = dir_path + strip_path;
    } else if path.starts_with("..") {
        panic!("not supported")
    } else if path == "." || path.starts_with("/") {
        path = dir_path;
    } else {
        path = dir_path + &path;
    }
    File::unlink(&path).unwrap();

    Ok(0)
}

pub(super) fn sys_write(task: &Sel4Task, fd: usize, buf: *const u8, len: usize) -> SysResult {
    let buf = task.read_bytes(buf as _, len).unwrap();

    let mut file_table = task.file.file_ds.lock();
    let file = file_table.get_mut(fd).ok_or(Errno::EBADF)?;

    file.lock().write(&buf)
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
    task.write_bytes(buf as _, task.file.work_dir.as_bytes());

    Ok(buf as _)
}

pub(super) fn sys_mkdirat(
    task: &Sel4Task,
    dirfd: isize,
    path: *const u8,
    mode: usize,
) -> SysResult {
    log::debug!(
        "mkdir at dirfd: {}  path: {:p}  mode: {:#x}",
        dirfd,
        path,
        mode
    );
    let path_bytes = task.read_cstr(path as _).unwrap();
    let mut path = String::from_utf8(path_bytes).unwrap();

    if dirfd == FD_CUR_DIR {
        path = task.file.work_dir.clone() + &path;
    }

    File::mkdir(&path)
}

pub(super) fn sys_openat(
    task: &mut Sel4Task,
    fd: isize,
    path: *const u8,
    flags: u64,
    _mode: usize,
) -> SysResult {
    let path_bytes = task.read_cstr(path as _).unwrap();
    let mut path = String::from_utf8(path_bytes).unwrap();
    log::debug!("open path: {}", path);

    let dir_path = if fd == FD_CUR_DIR {
        task.file.work_dir.clone()
    } else if fd > 0 {
        let dir = task
            .file
            .file_ds
            .lock()
            .get(fd as _)
            .ok_or(Errno::EBADF)?
            .clone();

        dir.lock().path() + "/"
    } else {
        panic!("not supported")
    };

    if let Some(strip_path) = path.strip_prefix("./") {
        path = dir_path + strip_path;
    } else if path.starts_with("..") {
        panic!("not supported")
    } else if path == "." || path.starts_with("/") {
        path = dir_path;
    } else {
        path = dir_path + &path;
    }

    let file = File::open(&path, flags)?;
    match task.file.file_ds.lock().add(Arc::new(Mutex::new(file))) {
        Ok(idx) => Ok(idx),
        Err(_) => todo!(),
    }
}
