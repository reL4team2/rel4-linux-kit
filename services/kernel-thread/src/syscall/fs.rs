//! 系统调用文件模块
//!
//!

use alloc::{boxed::Box, string::String, sync::Arc};
use common::services::fs::Stat;
use num_enum::TryFromPrimitive;
use spin::mutex::Mutex;
use syscalls::Errno;
use zerocopy::{FromBytes, IntoBytes};

use crate::{
    consts::fd::FD_CUR_DIR,
    fs::{file::File, get_mounted, mount, pipe::create_pipe2, umount},
    syscall::types::fs::FcntlCmd,
    task::Sel4Task,
};

use super::{SysResult, types::IoVec};

pub(super) fn sys_chdir(task: &mut Sel4Task, path: *const u8) -> SysResult {
    let path = task.deal_path(FD_CUR_DIR, path)?;
    const O_DIRECTORY: u64 = 0o40000;
    // 确保路径存在
    File::open(&path, O_DIRECTORY)?;
    task.file.work_dir = path;

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
    let mut buffer = vec![0u8; len];
    let rlen = file.lock().getdents64(&mut buffer)?;
    task.write_bytes(buf_ptr as _, &buffer[..rlen]);
    Ok(rlen)
}

pub(super) fn sys_pipe2(task: &Sel4Task, fdsp: *const u32, flags: u64) -> SysResult {
    if flags != 0 {
        panic!("flags != 0 is not supported");
    }
    log::debug!("pipe2 {:#p} {:#x}", fdsp, flags);
    let (rxp, txp) = create_pipe2();
    let mut file_table = task.file.file_ds.lock();

    let rx = file_table
        .add(Arc::new(Mutex::new(File::from_raw(Box::new(rxp)))))
        .map_err(|_| Errno::EMFILE)? as u32;
    let tx = file_table
        .add(Arc::new(Mutex::new(File::from_raw(Box::new(txp)))))
        .map_err(|_| Errno::EMFILE)? as u32;

    task.write_bytes(fdsp as _, [rx, tx].as_bytes());

    Ok(0)
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
    let path = task.deal_path(fd, path)?;
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
    log::warn!("mkdirat @ mod {} is not supported", mode);
    let path = task.deal_path(dirfd, path)?;
    File::mkdir(&path)
}

pub(super) fn sys_openat(
    task: &mut Sel4Task,
    fd: isize,
    path: *const u8,
    flags: u64,
    _mode: usize,
) -> SysResult {
    let path = task.deal_path(fd, path)?;

    let file = File::open(&path, flags)?;
    match task.file.file_ds.lock().add(Arc::new(Mutex::new(file))) {
        Ok(idx) => Ok(idx),
        Err(_) => todo!(),
    }
}

pub(super) fn sys_mount(
    task: &Sel4Task,
    source: *const u8,
    target: *const u8,
    fstype: *const u8,
    flags: u64,
    data: usize,
) -> SysResult {
    let source = String::from_utf8(task.read_cstr(source as _).ok_or(Errno::EINVAL)?).unwrap();
    let target = String::from_utf8(task.read_cstr(target as _).ok_or(Errno::EINVAL)?).unwrap();
    let fstype = String::from_utf8(task.read_cstr(fstype as _).ok_or(Errno::EINVAL)?).unwrap();
    log::debug!(
        "mount @ {} -> {} {} {:#x} {:#x}",
        source,
        target,
        fstype,
        flags,
        data
    );
    if source == "/dev/vda2" {
        mount(&target, get_mounted("/").1).unwrap();
    } else {
        return Err(Errno::EPERM);
    }
    Ok(0)
}

/// TODO: 检查 `arg` 参数，完善 `fcntl` 系统调用
pub(super) fn sys_fcntl(task: &Sel4Task, fd: usize, cmd: u32, _arg: usize) -> SysResult {
    let cmd = FcntlCmd::try_from_primitive(cmd).map_err(|_| Errno::EINVAL)?;

    // 检查文件是否存在
    let _file = task.file.file_ds.lock().get_mut(fd).ok_or(Errno::EBADF)?;
    match cmd {
        FcntlCmd::DUPFD | FcntlCmd::DUPFDCLOEXEC => sys_dup(task, fd),
        FcntlCmd::SETFD => Ok(0),
        _ => todo!("cmd is not implemented: {:?}", cmd),
    }
}

pub(super) fn sys_umount(task: &Sel4Task, target: *const u8, flags: u64) -> SysResult {
    let target = String::from_utf8(task.read_cstr(target as _).ok_or(Errno::EINVAL)?).unwrap();
    log::debug!("umount @ {} {:#x}", target, flags);
    umount(&target).map(|_| 0)
}
