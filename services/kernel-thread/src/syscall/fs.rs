//! 系统调用文件模块
//!
//!

use core::time::Duration;

use alloc::{string::String, sync::Arc};
use bit_field::BitArray;
use fs::{FileType, SeekFrom, file::File};
use libc_core::{
    consts::UTIME_NOW,
    fcntl::{AT_FDCWD, AT_SYMLINK_NOFOLLOW, FcntlCmd, OpenFlags},
    poll::{PollEvent, PollFd},
    types::{IoVec, Stat, StatFS, TimeSpec},
};
use num_enum::TryFromPrimitive;
use sel4_kit::arch::current_time;
use syscalls::Errno;
use zerocopy::{FromBytes, FromZeros, IntoBytes};

use crate::{fs::pipe::create_pipe, task::Sel4Task, timer::wait_time};

use super::SysResult;

pub(super) fn sys_chdir(task: &Sel4Task, path: *const u8) -> SysResult {
    let dir = task.fd_open(AT_FDCWD, path, OpenFlags::DIRECTORY)?;
    // 确保路径存在
    *task.file.work_dir.lock() = dir;

    Ok(0)
}

pub(super) fn sys_close(task: &Sel4Task, fd: usize) -> SysResult {
    task.file.file_ds.lock().remove(fd).ok_or(Errno::EBADF)?;
    Ok(0)
}

pub(super) fn sys_dup(task: &Sel4Task, fd: usize) -> SysResult {
    let mut file_table = task.file.file_ds.lock();
    let old_fd = file_table.get(fd).ok_or(Errno::EBADF)?.clone();
    if file_table.count() >= task.file.rlimit.lock().curr {
        return Err(Errno::EMFILE);
    }
    file_table.add(old_fd).map_err(|_| Errno::EMFILE)
}

pub(super) fn sys_dup3(task: &Sel4Task, fd: usize, fd_dst: usize) -> SysResult {
    let mut file_table = task.file.file_ds.lock();
    let old_fd = file_table.get(fd).ok_or(Errno::EBADF)?.clone();
    if file_table.count() >= task.file.rlimit.lock().curr && fd_dst >= file_table.count() {
        return Err(Errno::EMFILE);
    }
    let _ = file_table.remove(fd_dst);
    file_table.add_at(fd_dst, old_fd).map_err(|_| Errno::EMFILE)
}

pub(super) fn sys_fstat(task: &Sel4Task, fd: usize, stat_ptr: *mut Stat) -> SysResult {
    let file_table = task.file.file_ds.lock();
    let file = file_table.get(fd).ok_or(Errno::EBADF)?.clone();

    let mut stat = Stat::new_zeroed();
    file.stat(&mut stat)?;
    task.write_bytes(stat_ptr as _, stat.as_bytes());
    Ok(0)
}

pub(super) fn sys_fstatat(
    task: &Sel4Task,
    dirfd: isize,
    path_ptr: *const u8,
    stat_ptr: *mut Stat,
    flags: u32,
) -> SysResult {
    let path = task.fd_resolve(dirfd, path_ptr)?;
    let file = if flags & AT_SYMLINK_NOFOLLOW == 0 {
        File::open(path, OpenFlags::RDONLY)?
    } else {
        File::open_link(path, OpenFlags::RDONLY)?
    };
    let mut stat: Stat = Stat::default();
    file.stat(&mut stat)?;

    task.write_bytes(stat_ptr as _, stat.as_bytes());
    Ok(0)
}

pub(super) fn sys_statfs(
    task: &Sel4Task,
    filename_ptr: *const u8,
    statfs_ptr: *mut StatFS,
) -> SysResult {
    let mut statfs: StatFS = StatFS::default();
    task.fd_open(AT_FDCWD, filename_ptr, OpenFlags::RDONLY)?
        .statfs(&mut statfs)?;
    task.write_bytes(statfs_ptr as _, statfs.as_bytes());
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
        task.tid, fd, buf_ptr, len
    );
    let file_table = task.file.file_ds.lock();
    let file = file_table.get(fd).ok_or(Errno::EBADF)?.clone();
    let mut buffer = vec![0u8; len];
    let rlen = file.getdents(&mut buffer)?;
    task.write_bytes(buf_ptr as _, &buffer[..rlen]);
    Ok(rlen)
}

pub(super) fn sys_pipe2(task: &Sel4Task, fdsp: *const u32, flags: u64) -> SysResult {
    if flags != 0 {
        panic!("flags != 0 is not supported");
    }
    log::debug!("pipe2 {:#p} {:#x}", fdsp, flags);
    let (rxp, txp) = create_pipe();
    let mut file_table = task.file.file_ds.lock();

    let rx = file_table
        .add(File::new_dev(rxp))
        .map_err(|_| Errno::EMFILE)? as u32;
    let tx = file_table
        .add(File::new_dev(txp))
        .map_err(|_| Errno::EMFILE)? as u32;
    task.write_bytes(fdsp as _, [rx, tx].as_bytes());

    Ok(0)
}

pub(super) async fn sys_read(
    task: &Sel4Task,
    fd: usize,
    bufp: *const u8,
    count: usize,
) -> SysResult {
    if count == 0 {
        return Err(Errno::EINVAL);
    }
    let file = task
        .file
        .file_ds
        .lock()
        .get_mut(fd)
        .ok_or(Errno::EBADF)?
        .clone();
    let mut buffer = vec![0u8; count];
    let rlen = loop {
        let res = file.read(&mut buffer);
        if let Ok(rlen) = res {
            break rlen;
        } else if let Err(Errno::EAGAIN) = res {
            wait_time(current_time() + Duration::new(0, 1000000), task.tid).await?;
        } else {
            res?;
        }
    };
    task.write_bytes(bufp as _, &buffer);
    Ok(rlen)
}

pub(super) async fn sys_readv(
    task: &Sel4Task,
    fd: usize,
    iov: *const IoVec,
    iocnt: usize,
) -> SysResult {
    let mut rsize = 0;
    let iovec_bytes = task
        .read_bytes(iov as _, size_of::<IoVec>() * iocnt)
        .unwrap();

    let iovec = <[IoVec]>::ref_from_bytes_with_elems(&iovec_bytes, iocnt).unwrap();
    for item in iovec.iter() {
        let rlen_once = sys_read(task, fd, item.base as _, item.len).await?;
        rsize += rlen_once;
    }

    Ok(rsize)
}

pub(super) fn sys_pread64(
    task: &Sel4Task,
    fd: usize,
    buff_ptr: *const u8,
    len: usize,
    offset: usize,
) -> SysResult {
    let mut buffer = vec![0u8; len];
    let file = task
        .file
        .file_ds
        .lock()
        .get(fd)
        .ok_or(Errno::EBADF)?
        .clone();
    let rlen = file.readat(offset, &mut buffer)?;
    task.write_bytes(buff_ptr as _, &buffer[..rlen]);
    Ok(rlen)
}

pub(super) fn sys_unlinkat(task: &Sel4Task, fd: isize, path: *const u8, _flags: u64) -> SysResult {
    task.fd_open(fd, path, OpenFlags::RDONLY)?.remove_self()?;
    Ok(0)
}

pub(super) fn sys_write(task: &Sel4Task, fd: usize, buf: *const u8, len: usize) -> SysResult {
    let buf = task.read_bytes(buf as _, len).unwrap();

    let mut file_table = task.file.file_ds.lock();
    let file = file_table.get_mut(fd).ok_or(Errno::EBADF)?;

    file.write(&buf)
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

pub(super) fn sys_pwrite64(
    task: &Sel4Task,
    fd: usize,
    buf_ptr: *const u8,
    count: usize,
    offset: usize,
) -> SysResult {
    let buf = task.read_bytes(buf_ptr as _, count).unwrap();
    task.file
        .file_ds
        .lock()
        .get_mut(fd)
        .ok_or(Errno::EBADF)?
        .writeat(offset, &buf)
}

pub(super) fn sys_lseek(task: &Sel4Task, fd: usize, offset: usize, whence: usize) -> SysResult {
    let seek_from = match whence {
        0 => SeekFrom::SET(offset),
        1 => SeekFrom::CURRENT(offset as isize),
        2 => SeekFrom::END(offset as isize),
        _ => return Err(Errno::EINVAL),
    };

    task.file
        .file_ds
        .lock()
        .get(fd)
        .ok_or(Errno::EBADF)?
        .seek(seek_from)
}

pub(super) fn sys_getcwd(task: &Sel4Task, buf: *mut u8, _size: usize) -> SysResult {
    log::warn!("get cwd is a simple implement, always return /");
    task.write_bytes(buf as _, task.file.work_dir.lock().path().as_bytes());

    Ok(buf as _)
}

pub(super) fn sys_mkdirat(
    task: &Sel4Task,
    dirfd: isize,
    path: *const u8,
    mode: usize,
) -> SysResult {
    log::warn!("mkdirat @ mod {} is not supported", mode);
    task.fd_open(dirfd, path, OpenFlags::DIRECTORY | OpenFlags::CREAT)?;
    Ok(0)
}

pub(super) fn sys_openat(
    task: &Sel4Task,
    fd: isize,
    path: *const u8,
    flags: usize,
    _mode: usize,
) -> SysResult {
    let flags = OpenFlags::from_bits_truncate(flags);
    let file = task.fd_open(fd, path, flags)?;

    if task.file.file_ds.lock().count() >= task.file.rlimit.lock().curr {
        return Err(Errno::EMFILE);
    }

    match task.file.file_ds.lock().add(Arc::new(file)) {
        Ok(idx) => Ok(idx),
        Err(_) => Err(Errno::EMFILE),
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
        // mount(&target, get_mounted("/").1).unwrap();
    } else {
        return Err(Errno::EPERM);
    }
    Ok(0)
}

/// TODO: 检查 `arg` 参数，完善 `fcntl` 系统调用
pub(super) fn sys_fcntl(task: &Sel4Task, fd: usize, cmd: u32, arg: usize) -> SysResult {
    let cmd = FcntlCmd::try_from_primitive(cmd).map_err(|_| Errno::EINVAL)?;
    // 检查文件是否存在
    let file = task
        .file
        .file_ds
        .lock()
        .get_mut(fd)
        .ok_or(Errno::EBADF)?
        .clone();
    match cmd {
        FcntlCmd::DUPFD | FcntlCmd::DUPFDCLOEXEC => sys_dup(task, fd),
        FcntlCmd::SETFD => Ok(0),
        FcntlCmd::GETFL => Ok(file.flags.lock().bits()),
        FcntlCmd::SETFL => {
            let mut file_table = task.file.file_ds.lock();
            *file.flags.lock() = OpenFlags::from_bits_truncate(arg);
            let _ = file_table.remove(fd);
            file_table.add_at(fd, file).map_err(|_| Errno::EMFILE)?;
            Ok(0)
        }
        _ => todo!("cmd is not implemented: {:?}", cmd),
    }
}

pub(super) fn sys_umount(task: &Sel4Task, target: *const u8, flags: u64) -> SysResult {
    let target = String::from_utf8(task.read_cstr(target as _).ok_or(Errno::EINVAL)?).unwrap();
    log::debug!("umount @ {} {:#x}", target, flags);
    // umount(&target).map(|_| 0)
    Ok(0)
}

pub(super) fn sys_utimensat(
    task: &Sel4Task,
    dirfd: isize,
    path: *const u8,
    times_ptr: *mut TimeSpec,
    _flags: usize,
) -> SysResult {
    // build times
    let mut times = match times_ptr.is_null() {
        true => {
            vec![current_time().into(), current_time().into()]
        }
        false => {
            let timespec_bytes = task
                .read_bytes(times_ptr as _, size_of::<TimeSpec>() * 2)
                .ok_or(Errno::EINVAL)?;
            let ts = <[TimeSpec]>::ref_from_bytes_with_elems(&timespec_bytes, 2)
                .map_err(|_| Errno::EINVAL)?;
            let mut times = vec![];
            for item in ts.iter().take(2) {
                if item.nsec == UTIME_NOW {
                    times.push(current_time().into());
                } else {
                    times.push(*item);
                }
            }
            times.to_vec()
        }
    };

    if path.is_null() {
        task.file
            .file_ds
            .lock()
            .get(dirfd as _)
            .ok_or(Errno::EBADF)?
            .utimes(&mut times)?;
        return Ok(0);
    }

    // debug!("times: {:?} path: {}", times, path);
    // if path == "/dev/null/invalid" {
    //     return Ok(0);
    // }
    task.fd_open(dirfd, path, OpenFlags::RDONLY)?
        .utimes(&mut times)?;

    Ok(0)
}

pub(super) fn sys_sendfile(
    task: &Sel4Task,
    out_fd: usize,
    in_fd: usize,
    offset: usize,
    count: usize,
) -> SysResult {
    debug!(
        "sys_sendfile @ out_fd: {}  in_fd: {}  offset: {:#x}   count: {:#x}",
        out_fd, in_fd, offset, count
    );
    let outfile = task
        .file
        .file_ds
        .lock()
        .get(out_fd)
        .ok_or(Errno::EINVAL)?
        .clone();
    let infile = task
        .file
        .file_ds
        .lock()
        .get(in_fd)
        .ok_or(Errno::EINVAL)?
        .clone();

    let curr_off = if offset != 0 {
        offset
    } else {
        infile.seek(SeekFrom::CURRENT(0))?
    };
    let rlen = core::cmp::min(infile.file_size()? - curr_off, count);
    let mut buffer = vec![0u8; rlen];

    if offset == 0 {
        infile.read(&mut buffer)?;
        let _ = task.file.file_ds.lock().add_or_replace_at(in_fd, infile);
    } else {
        infile.readat(offset, &mut buffer)?;
    }
    outfile.write(&buffer)
}

pub(super) async fn sys_ppoll(
    task: &Sel4Task,
    poll_fds_ptr: *const PollFd,
    nfds: usize,
    timeout_ptr: *const TimeSpec,
    sigmask_ptr: usize,
) -> SysResult {
    debug!(
        "sys_ppoll @ poll_fds_ptr: {:p}, nfds: {}, timeout_ptr: {:p}, sigmask_ptr: {:#X}",
        poll_fds_ptr, nfds, timeout_ptr, sigmask_ptr
    );
    let mut poll_fds_bytes = task
        .read_bytes(poll_fds_ptr as _, size_of::<PollFd>() * nfds)
        .unwrap();
    let poll_fds = <[PollFd]>::mut_from_bytes_with_elems(&mut poll_fds_bytes, nfds).unwrap();
    let etime = if !timeout_ptr.is_null() {
        let timeout_bytes = task
            .read_bytes(timeout_ptr as _, size_of::<TimeSpec>())
            .ok_or(Errno::EINVAL)?;
        current_time() + TimeSpec::read_from_bytes(&timeout_bytes).unwrap().into()
    } else {
        Duration::MAX
    };
    let n = loop {
        let mut num = 0;
        for poll_fd in poll_fds.iter_mut().take(nfds) {
            poll_fd.revents = task
                .file
                .file_ds
                .lock()
                .get(poll_fd.fd as _)
                .map_or(PollEvent::NONE, |x| x.poll(poll_fd.events.clone()).unwrap());
            if poll_fd.revents != PollEvent::NONE {
                num += 1;
            }
        }

        if current_time() >= etime || num > 0 {
            break num;
        }

        wait_time(current_time() + Duration::new(0, 1000000), task.tid).await?;
    };
    task.write_bytes(poll_fds_ptr as _, poll_fds.as_bytes());
    Ok(n)
}

pub(super) fn sys_faccessat(
    task: &Sel4Task,
    dir_fd: isize,
    filename: *const u8,
    _mode: usize,
    flags: usize,
) -> SysResult {
    let open_flags = OpenFlags::from_bits_truncate(flags);
    let path = task.fd_resolve(dir_fd, filename)?;
    log::debug!("sys_faccessat @ open path: {}", path);
    File::open(path, open_flags)?;
    Ok(0)
}

pub(super) fn sys_renameat2(
    task: &Sel4Task,
    olddir_fd: isize,
    oldpath: *const u8,
    newdir_fd: isize,
    newpath: *const u8,
    flags: usize,
) -> SysResult {
    debug!(
        "sys_renameat2 @ olddir_fd: {}, oldpath: {:p}, newdir_fd: {}, newpath: {:p}, flags: {}",
        olddir_fd, oldpath, newdir_fd, newpath, flags
    );
    let flags = OpenFlags::from_bits_truncate(flags);

    let old_file = task.fd_open(olddir_fd, oldpath, OpenFlags::RDONLY)?;

    let old_file_type = old_file.file_type()?;

    if old_file_type == FileType::File {
        let new_file = task.fd_open(newdir_fd, newpath, OpenFlags::CREAT | flags)?;
        let file_size = old_file.file_size()?;
        let mut buffer = vec![0u8; file_size];
        old_file.read(&mut buffer)?;
        new_file.write(&buffer)?;
        new_file.truncate(buffer.len())?;
    } else if old_file_type == FileType::Directory {
        task.fd_open(
            newdir_fd,
            newpath,
            OpenFlags::CREAT | OpenFlags::DIRECTORY | flags,
        )?;
    } else {
        panic!("can't handle the file: {:?} now", old_file_type);
    }

    Ok(0)
}

pub(super) fn sys_ioctl(
    task: &Sel4Task,
    fd: usize,
    request: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> SysResult {
    debug!(
        "[task {}] ioctl: fd: {}, request: {:#x}, args: {:#x} {:#x} {:#x}",
        task.tid, fd, request, arg1, arg2, arg3
    );
    task.file
        .file_ds
        .lock()
        .get(fd)
        .ok_or(Errno::EINVAL)?
        .ioctl(request, arg1)
        .map_err(|_| Errno::ENOTTY)
}

pub(super) fn sys_ftruncate(task: &Sel4Task, fd: usize, len: usize) -> SysResult {
    if fd == usize::MAX {
        return Err(Errno::EPERM);
    }
    task.file
        .file_ds
        .lock()
        .get(fd)
        .ok_or(Errno::EINVAL)?
        .truncate(len)?;
    Ok(0)
}

#[allow(warnings)]
pub(super) async fn sys_pselect(
    task: &Sel4Task,
    mut max_fdp1: usize,
    readfds_ptr: *mut usize,
    writefds_ptr: *mut usize,
    exceptfds_ptr: *mut usize,
    timeout_ptr: *mut TimeSpec,
    sigmask: usize,
) -> SysResult {
    debug!(
        "[task {}] sys_pselect @ max_fdp1: {}, readfds: {:p}, writefds: {:p}, exceptfds: {:p}, tsptr: {:p}, sigmask: {:#X}",
        task.tid, max_fdp1, readfds_ptr, writefds_ptr, exceptfds_ptr, timeout_ptr, sigmask
    );

    max_fdp1 = core::cmp::min(max_fdp1, 255);

    let timeout = if timeout_ptr.is_null() {
        Duration::MAX
    } else {
        let timeout_bytes = task
            .read_bytes(timeout_ptr as _, size_of::<TimeSpec>())
            .ok_or(Errno::EINVAL)?;
        current_time() + TimeSpec::read_from_bytes(&timeout_bytes).unwrap().into()
    };
    let bytes = max_fdp1.div_ceil(8);
    let mut ori_rfds = if !readfds_ptr.is_null() {
        task.read_bytes(readfds_ptr as _, bytes).unwrap_or_default()
    } else {
        vec![0u8; bytes]
    };
    let mut ori_wfds = if !writefds_ptr.is_null() {
        task.read_bytes(writefds_ptr as _, bytes)
            .unwrap_or_default()
    } else {
        vec![0u8; bytes]
    };
    let mut ori_efds = if !exceptfds_ptr.is_null() {
        task.read_bytes(exceptfds_ptr as _, bytes)
            .unwrap_or_default()
    } else {
        vec![0u8; bytes]
    };
    let mut rfds_r = [0u8; 32];
    let mut wfds_r = [0u8; 32];
    let mut efds_r = [0u8; 32];
    loop {
        let mut num = 0;
        let file_ds = task.file.file_ds.lock();
        if !readfds_ptr.is_null() {
            for i in 0..max_fdp1 {
                if !ori_rfds.get_bit(i) {
                    rfds_r.set_bit(i, false);
                    continue;
                }
                if ori_rfds.get(i).is_none() {
                    rfds_r.set_bit(i, false);
                    continue;
                }
                let file = file_ds.get(i).unwrap();
                match file.poll(PollEvent::IN) {
                    Ok(res) => {
                        if res.contains(PollEvent::IN) {
                            num += 1;
                            rfds_r.set_bit(i, true);
                        } else {
                            rfds_r.set_bit(i, false)
                        }
                    }
                    Err(_) => rfds_r.set_bit(i, false),
                }
            }
        }
        if !writefds_ptr.is_null() {
            for i in 0..max_fdp1 {
                if !ori_wfds.get_bit(i) {
                    continue;
                }
                if ori_wfds.get(i).is_none() {
                    wfds_r.set_bit(i, false);
                    continue;
                }
                let file = file_ds.get(i).unwrap();
                match file.poll(PollEvent::OUT) {
                    Ok(res) => {
                        if res.contains(PollEvent::OUT) {
                            num += 1;
                            wfds_r.set_bit(i, true);
                        } else {
                            wfds_r.set_bit(i, false);
                        }
                    }
                    Err(_) => wfds_r.set_bit(i, false),
                }
            }
        }
        if !exceptfds_ptr.is_null() {
            for i in 0..max_fdp1 {
                if !ori_efds.get_bit(i) {
                    continue;
                }
                if ori_efds.get(i).is_none() {
                    efds_r.set_bit(i, false);
                    continue;
                }
                let file = file_ds.get(i).unwrap();
                match file.poll(PollEvent::ERR) {
                    Ok(res) => {
                        if res.contains(PollEvent::ERR) {
                            num += 1;
                            efds_r.set_bit(i, true);
                        } else {
                            efds_r.set_bit(i, false)
                        }
                    }
                    Err(_) => efds_r.set_bit(i, false),
                }
            }
        }
        drop(file_ds);
        if num != 0 {
            if !readfds_ptr.is_null() {
                task.write_bytes(readfds_ptr as _, &rfds_r[..bytes]);
            }
            if !writefds_ptr.is_null() {
                task.write_bytes(writefds_ptr as _, &wfds_r[..bytes]);
            }
            if !exceptfds_ptr.is_null() {
                task.write_bytes(exceptfds_ptr as _, &efds_r[..bytes]);
            }
            return Ok(num);
        }

        if current_time() > timeout {
            if !readfds_ptr.is_null() {
                task.write_bytes(readfds_ptr as _, &rfds_r[..bytes]);
            }
            if !writefds_ptr.is_null() {
                task.write_bytes(writefds_ptr as _, &wfds_r[..bytes]);
            }
            if !exceptfds_ptr.is_null() {
                task.write_bytes(exceptfds_ptr as _, &efds_r[..bytes]);
            }
            return Ok(0);
        }
    }
}
