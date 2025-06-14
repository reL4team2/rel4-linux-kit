//! 信号相关的系统调用
//!
//!

use libc_core::{
    internal::SigAction,
    signal::SignalNum,
    types::{SigMaskHow, SigSet},
};
use sel4::UserContext;
use syscalls::Errno;
use zerocopy::{FromBytes, IntoBytes};

use crate::task::Sel4Task;

use super::SysResult;

pub(super) fn sys_sigprocmask(
    task: &Sel4Task,
    how: u8,
    set: *const SigSet,
    old: *mut SigSet,
) -> SysResult {
    if !old.is_null() {
        task.write_bytes(old as _, task.signal.lock().mask.as_bytes());
    }
    if !set.is_null() {
        let sigproc_bytes = task.read_bytes(set as _, size_of::<SigSet>()).unwrap();
        let maskset = &mut SigSet::ref_from_bytes(&sigproc_bytes).unwrap();
        let sighow = SigMaskHow::try_from(how).or(Err(Errno::EINVAL))?;
        task.signal.lock().mask.handle(sighow, maskset);
    }
    Ok(0)
}

pub(super) fn sys_sigaction(
    task: &Sel4Task,
    sig: usize,
    act: *const SigAction,
    oldact: *mut SigAction,
) -> SysResult {
    if !oldact.is_null() {
        task.write_bytes(
            oldact as _,
            task.signal.lock().actions.lock()[sig].as_bytes(),
        );
    }

    if !act.is_null() {
        let sigaction_bytes = task.read_bytes(act as _, size_of::<SigAction>()).unwrap();
        let sigact = SigAction::ref_from_bytes(&sigaction_bytes).unwrap();
        task.signal.lock().actions.lock()[sig] = sigact.clone();
    }
    Ok(0)
}

pub(super) fn sys_kill(task: &Sel4Task, pid: usize, sig: usize) -> SysResult {
    assert_eq!(pid, task.pid);
    task.add_signal(SignalNum::from_num(sig).ok_or(Errno::EINVAL)?, task.tid);
    Ok(0)
}

pub(super) fn sys_sigreturn(task: &Sel4Task, ctx: &mut UserContext) -> SysResult {
    task.read_ucontext(ctx);
    *ctx.pc_mut() -= 4;
    Ok(*ctx.c_param(0) as _)
}

pub(super) fn sys_sigtimedwait(_task: &Sel4Task) -> SysResult {
    debug!("sys_sigtimedwait @ ");
    // WaitSignal(self.task.clone()).await;
    // let task = current_user_task();
    // task.inner_map(|x| x.signal.has_signal());
    // Err(LinuxError::EAGAIN)
    Ok(0)
}
