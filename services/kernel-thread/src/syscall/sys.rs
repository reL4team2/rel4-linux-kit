//! 系统信息系统调用
//!
//!

use zerocopy::{FromBytes, IntoBytes};

use crate::task::Sel4Task;

use super::{
    types::sys::{TimeVal, UtsName},
    SysResult,
};

pub(super) fn sys_uname(task: &mut Sel4Task, buf: *mut UtsName) -> SysResult {
    let mut utsname_bytes = task.read_bytes(buf as _, size_of::<UtsName>()).unwrap();
    let utsname = UtsName::mut_from_bytes(&mut utsname_bytes).unwrap();
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
    task.write_bytes(buf as _, &utsname.as_bytes()).unwrap();
    Ok(0)
}

pub(super) fn sys_gettimeofday(
    task: &mut Sel4Task,
    tv: *mut TimeVal,
    _timeone: usize,
) -> SysResult {
    let tv_now = TimeVal::now();
    task.write_bytes(tv as _, tv_now.as_bytes());
    Ok(0)
}
