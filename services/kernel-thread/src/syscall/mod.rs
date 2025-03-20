//! 系统调用处理模块
//!
//!
pub mod fs;
pub mod mm;
pub mod signal;
pub mod sys;
pub mod thread;
pub mod types;

use fs::*;
use mm::*;
use sel4::UserContext;
use signal::*;
use sys::*;
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
    let id = Sysno::new(*ctx.gpr(8) as _);
    let a0 = *ctx.gpr(0) as usize;
    let a1 = *ctx.gpr(1) as usize;
    let a2 = *ctx.gpr(2) as usize;
    let a3 = *ctx.gpr(3) as usize;
    let a4 = *ctx.gpr(4) as usize;
    let a5 = *ctx.gpr(5) as usize;
    log::debug!("SysCall: {:#x?}", id.unwrap());
    if id.is_none() {
        return Err(Errno::ENOSYS);
    }
    match id.unwrap() {
        Sysno::brk => sys_brk(task, a0),
        Sysno::chdir => sys_chdir(task, a0 as _),
        Sysno::clone => sys_clone(task, a0 as _, a1, a2 as _, a3, a4 as _),
        Sysno::close => sys_close(task, a0),
        Sysno::dup => sys_dup(task, a0),
        Sysno::dup3 => sys_dup3(task, a0, a1),
        Sysno::execve => sys_execve(task, ctx, a0 as _, a1 as _, a2 as _),
        Sysno::exit => sys_exit(task, a0 as _),
        Sysno::fcntl => sys_fcntl(task, a0, a1 as _, a2 as _),
        Sysno::fstat => sys_fstat(task, a0, a1 as _),
        Sysno::getcwd => sys_getcwd(task, a0 as _, a1),
        Sysno::getdents64 => sys_getdents64(task, a0, a1 as _, a2),
        Sysno::getpid => sys_getpid(task),
        Sysno::getppid => sys_getppid(task),
        Sysno::gettimeofday => sys_gettimeofday(task, a0 as _, a1),
        Sysno::kill => sys_kill(task, a0, a1),
        Sysno::mkdirat => sys_mkdirat(task, a0 as _, a1 as _, a2),
        Sysno::mmap => sys_mmap(task, a0, a1, a2, a3, a4 as _, a5),
        Sysno::mount => sys_mount(task, a0 as _, a1 as _, a2 as _, a3 as _, a4),
        Sysno::munmap => sys_munmap(task, a0, a1),
        Sysno::nanosleep => sys_nanosleep(task, a0 as _, a1 as _),
        Sysno::openat => sys_openat(task, a0 as _, a1 as _, a2 as _, a3),
        Sysno::pipe2 => sys_pipe2(task, a0 as _, a1 as _),
        Sysno::read => sys_read(task, a0, a1 as _, a2),
        Sysno::rt_sigaction => sys_sigaction(task, a0, a1 as _, a2 as _),
        Sysno::rt_sigprocmask => sys_sigprocmask(task, a0, a1 as _, a2 as _),
        Sysno::rt_sigreturn => sys_sigreturn(task, ctx),
        Sysno::sched_yield => sys_sched_yield(task),
        Sysno::set_tid_address => sys_set_tid_addr(task, a0),
        Sysno::umount2 => sys_umount(task, a0 as _, a1 as _),
        Sysno::uname => sys_uname(task, a0 as _),
        Sysno::unlinkat => sys_unlinkat(task, a0 as _, a1 as _, a2 as _),
        Sysno::wait4 => sys_wait4(task, ctx, a0 as _, a1 as _, a2 as _),
        Sysno::write => sys_write(task, a0, a1 as _, a2),
        Sysno::writev => sys_writev(task, a0, a1 as _, a2),
        Sysno::getuid | Sysno::getgid | Sysno::ioctl => Ok(0),
        _ => Err(Errno::EPERM),
    }
}
