//! 信号相关的系统调用
//!
//!

use sel4::UserContext;
use syscalls::Errno;
use zerocopy::FromBytes;

use crate::task::Sel4Task;

use super::{
    types::signal::{SigAction, SigMaskHow, SigProcMask},
    SysResult,
};

pub(super) fn sys_sigprocmask(
    task: &mut Sel4Task,
    how: usize,
    set: *const SigProcMask,
    old: *mut SigProcMask,
) -> SysResult {
    let sigproc_bytes = task.read_bytes(set as _, size_of::<SigProcMask>()).unwrap();
    let maskset = &mut SigProcMask::ref_from_bytes(&sigproc_bytes).unwrap();
    let sighow = SigMaskHow::try_from(how).or(Err(Errno::EINVAL))?;
    task.signal.mask.handle(sighow, maskset);
    warn!("write old sigmask is not implemented");
    Ok(0)
}

pub(super) fn sys_sigaction(
    task: &mut Sel4Task,
    sig: usize,
    act: *const SigAction,
    oldact: *mut SigAction,
) -> SysResult {
    let sigaction_bytes = task.read_bytes(act as _, size_of::<SigAction>()).unwrap();
    let sigact = SigAction::ref_from_bytes(&sigaction_bytes).unwrap();
    task.signal.actions[sig] = Some(sigact.clone());
    log::debug!("write old sigaction is not implemented");
    Ok(0)
}

pub(super) fn sys_kill(task: &mut Sel4Task, pid: usize, sig: usize) -> SysResult {
    assert_eq!(pid, task.pid);
    task.signal.pedings.push_back(sig as _);
    Ok(0)
}

pub(super) fn sys_sigreturn(task: &mut Sel4Task, ctx: &mut UserContext) -> SysResult {
    let saved_ctx = task.signal.save_context.pop().unwrap();
    *ctx = saved_ctx;
    Ok(ctx.c_param(0).clone() as _)
}
