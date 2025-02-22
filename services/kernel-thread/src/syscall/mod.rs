//! 系统调用处理模块
//!
//!
pub mod fs;
pub mod mm;
pub mod thread;
pub mod types;

use fs::*;
use mm::*;
use sel4::UserContext;
use syscalls::{Errno, Sysno};
use thread::*;

use crate::task::Sel4Task;

/// SysCall Result
///
/// 使用 `Errno` 作为错误类型，`usize` 作为返回值类型
pub type SysResult = Result<usize, Errno>;

/// 处理系统调用
/// - `task` [Sel4Task]   需要处理的任务
/// - `ctx` [UserContext] 系统调用上下文，修改后需要恢复
pub fn handle_syscall(task: &mut Sel4Task, ctx: &mut UserContext) -> SysResult {
    let id = Sysno::new(ctx.gpr(8).clone() as _);
    let a0 = ctx.gpr(0).clone() as usize;
    let a1 = ctx.gpr(1).clone() as usize;
    let a2 = ctx.gpr(2).clone() as usize;
    let a3 = ctx.gpr(3).clone() as usize;
    let a4 = ctx.gpr(4).clone() as usize;
    let a5 = ctx.gpr(5).clone() as usize;
    log::debug!("SysCall: {:#x?}", id.unwrap());
    if id == None {
        return Err(Errno::ENOSYS);
    }
    match id.unwrap() {
        Sysno::brk => sys_brk(task, a0),
        Sysno::mmap => sys_mmap(task, a0, a1, a2, a3, a4, a5),
        Sysno::getpid => sys_getpid(&task),
        Sysno::set_tid_address => sys_set_tid_addr(task, a0),
        Sysno::write => sys_write(task, a0, a1 as _, a2),
        Sysno::writev => sys_writev(task, a0, a1 as _, a2),
        Sysno::exit => panic!("exit is not implemented"),
        Sysno::rt_sigprocmask | Sysno::rt_sigaction | Sysno::getuid | Sysno::getgid => Ok(0),
        _ => Err(Errno::EPERM),
    }
}
