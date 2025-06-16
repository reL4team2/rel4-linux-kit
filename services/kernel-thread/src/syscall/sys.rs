//! 系统信息系统调用
//!
//!

use core::time::Duration;

use libc_core::{
    resource::Rlimit,
    types::{TimeSpec, TimeVal},
    utsname::UTSName,
};
use sel4_kit::arch::current_time;
use syscalls::Errno;
use zerocopy::{FromBytes, IntoBytes};

use crate::{task::Sel4Task, timer::wait_time};

use super::SysResult;

pub(super) fn sys_uname(task: &Sel4Task, buf: *mut UTSName) -> SysResult {
    let mut utsname_bytes = task.read_bytes(buf as _, size_of::<UTSName>()).unwrap();
    let utsname = UTSName::mut_from_bytes(&mut utsname_bytes).unwrap();
    let sysname = b"rel4-linux";
    let nodename = b"rel4-beta1";
    let release = b"vb0.1";
    let version = b"vb0.1";
    let machine = b"aarch64";
    utsname.sysname[..sysname.len()].copy_from_slice(sysname);
    utsname.nodename[..nodename.len()].copy_from_slice(nodename);
    utsname.release[..release.len()].copy_from_slice(release);
    utsname.version[..version.len()].copy_from_slice(version);
    utsname.machine[..machine.len()].copy_from_slice(machine);
    task.write_bytes(buf as _, utsname.as_bytes()).unwrap();
    Ok(0)
}

pub(super) fn sys_gettimeofday(task: &Sel4Task, tv: *mut TimeVal, _timeone: usize) -> SysResult {
    let tv_now: TimeVal = current_time().into();
    task.write_bytes(tv as _, tv_now.as_bytes());
    Ok(0)
}

pub(super) fn sys_clock_gettime(
    task: &Sel4Task,
    clock_id: usize,
    times_ptr: *mut TimeSpec,
) -> SysResult {
    debug!(
        "[task {}] sys_clock_gettime @ clock_id: {}, times_ptr: {:p}",
        task.pid, clock_id, times_ptr
    );

    let dura = match clock_id {
        0 => current_time(), // CLOCK_REALTIME
        1 => current_time(), // CLOCK_MONOTONIC
        2 => {
            warn!("CLOCK_PROCESS_CPUTIME_ID not implemented");
            Duration::ZERO
        }
        3 => {
            warn!("CLOCK_THREAD_CPUTIME_ID not implemented");
            Duration::ZERO
        }
        _ => return Err(Errno::EINVAL),
    };
    log::debug!("dura: {:#x?}", dura);
    let timespec: TimeSpec = dura.into();
    task.write_bytes(times_ptr as _, timespec.as_bytes());
    Ok(0)
}

pub(super) async fn sys_nanosleep(
    task: &Sel4Task,
    req_ptr: *const TimeSpec,
    rem_ptr: *mut TimeSpec,
) -> SysResult {
    debug!(
        "[task {}] sys_nanosleep @ req_ptr: {:p}, rem_ptr: {:p}",
        task.tid, req_ptr, rem_ptr
    );
    let curr_time = current_time();
    let nano_bytes = task
        .read_bytes(req_ptr as _, size_of::<TimeSpec>())
        .unwrap();
    let req = TimeSpec::ref_from_bytes(&nano_bytes).unwrap();
    debug!("nano sleep {} nseconds", req.sec * 1_000_000_000 + req.nsec);

    wait_time(
        curr_time + Duration::new(req.sec as _, req.nsec as _),
        task.tid,
    )
    .await?;

    if !rem_ptr.is_null() {
        task.write_bytes(rem_ptr as _, TimeSpec::default().as_bytes());
    }

    Ok(0)
}

pub(super) fn sys_prlimit64(
    task: &Sel4Task,
    pid: usize,
    resource: usize,
    new_limit: *const Rlimit,
    old_limit: *mut Rlimit,
) -> SysResult {
    debug!(
        "sys_getrlimit @ pid: {}, resource: {}, new_limit: {:p}, old_limit: {:p}",
        pid, resource, new_limit, old_limit
    );
    match resource {
        7 => {
            if !old_limit.is_null() {
                task.write_bytes(old_limit as _, task.file.rlimit.lock().as_bytes());
            }
            if !new_limit.is_null() {
                let rlimit_bytes = task
                    .read_bytes(new_limit as _, size_of::<Rlimit>())
                    .ok_or(Errno::EINVAL)?;
                let rlimit = Rlimit::read_from_bytes(&rlimit_bytes).unwrap();
                *task.file.rlimit.lock() = rlimit;
            }
        }
        _ => {
            warn!("need to finish prlimit64: resource {}", resource)
        }
    }
    Ok(0)
}
