//! 系统调用处理模块
//!
//!
pub mod fs;

use fs::{sys_write, sys_writev};
use sel4::UserContext;
use syscalls::{Errno, Sysno};

use crate::task::Sel4Task;

/// SysCall Result
///
/// 使用 `Errno` 作为错误类型，`usize` 作为返回值类型
pub type SysResult = Result<usize, Errno>;

/// 处理系统调用
/// - `task` [Sel4Task]   需要处理的任务
/// - `ctx` [UserContext] 系统调用上下文，修改后需要恢复
pub fn handle_syscall(task: &Sel4Task, ctx: &mut UserContext) -> SysResult {
    let id = Sysno::new(ctx.gpr(8).clone() as _);
    let a0 = ctx.gpr(0).clone() as usize;
    let a1 = ctx.gpr(1).clone() as usize;
    let a2 = ctx.gpr(2).clone() as usize;
    log::debug!(" {:08x} >> Received syscall: {:#x?}", ctx.pc(), id.unwrap());
    if id == None {
        return Err(Errno::ENOSYS);
    }
    match id.unwrap() {
        Sysno::write => sys_write(task, a0, a1 as _, a2),
        Sysno::writev => sys_writev(task, a0, a1 as _, a2),
        Sysno::exit => panic!("exit is not implemented"),
        _ => Err(Errno::EPERM),
    }
}
