//! 系统调用处理模块
//!
//!
pub mod fs;
pub mod mm;
pub mod signal;
pub mod sys;
pub mod thread;

use fs::*;
use libc_core::fcntl::OpenFlags;
use mm::*;
use sel4::UserContext;
use signal::*;
use sys::*;
use syscalls::{Errno, Sysno};
use thread::*;

use crate::child_test::ArcTask;

/// SysCall Result
///
/// 使用 `Errno` 作为错误类型，`usize` 作为返回值类型
pub type SysResult = Result<usize, Errno>;

/// 处理系统调用
/// - `task` [Sel4Task]   需要处理的任务
/// - `ctx` [UserContext] 系统调用上下文，修改后需要恢复
pub async fn handle_syscall(task: &ArcTask, ctx: &mut UserContext) -> SysResult {
    let id = Sysno::new(*ctx.gpr(8) as _);
    let a0 = *ctx.gpr(0) as usize;
    let a1 = *ctx.gpr(1) as usize;
    let a2 = *ctx.gpr(2) as usize;
    let a3 = *ctx.gpr(3) as usize;
    let a4 = *ctx.gpr(4) as usize;
    let a5 = *ctx.gpr(5) as usize;
    log::debug!("[task {}] SysCall: {:#x?}", task.tid, id.unwrap());
    if id.is_none() {
        return Err(Errno::ENOSYS);
    }
    match id.unwrap() {
        Sysno::brk => sys_brk(task, a0),
        Sysno::chdir => sys_chdir(task, a0 as _),
        Sysno::clone => sys_clone(task, a0 as _, a1, a2 as _, a3, a4 as _).await,
        Sysno::close => sys_close(task, a0),
        Sysno::dup => sys_dup(task, a0),
        Sysno::dup3 => sys_dup3(task, a0, a1),
        Sysno::execve => sys_execve(task, ctx, a0 as _, a1 as _, a2 as _),
        Sysno::exit => sys_exit(task, a0 as _),
        Sysno::faccessat => sys_faccessat(task, a0 as _, a1 as _, a2 as _, a3 as _),
        Sysno::fcntl => sys_fcntl(task, a0, a1 as _, a2 as _),
        Sysno::fstat => sys_fstat(task, a0, a1 as _),
        Sysno::fstatat => sys_fstatat(task, a0 as _, a1 as _, a2 as _, a3 as _),
        Sysno::ftruncate => sys_ftruncate(task, a0, a1 as _),
        Sysno::statfs => sys_statfs(task, a0 as _, a1 as _),
        Sysno::futex => sys_futex(task.clone(), a0 as _, a1, a2, a3, a4, a5).await,
        Sysno::getcwd => sys_getcwd(task, a0 as _, a1),
        Sysno::getdents64 => sys_getdents64(task, a0, a1 as _, a2),
        Sysno::getpid => sys_getpid(task),
        Sysno::getppid => sys_getppid(task),
        Sysno::gettid => sys_gettid(task),
        Sysno::getrusage => sys_getrusage(task, a0, a1 as _),
        Sysno::lseek => sys_lseek(task, a0 as _, a1 as _, a2 as _),
        Sysno::ioctl => sys_ioctl(task, a0, a1, a2, a3, a4),
        Sysno::clock_gettime => sys_clock_gettime(task, a0 as _, a1 as _),
        Sysno::gettimeofday => sys_gettimeofday(task, a0 as _, a1),
        Sysno::kill => sys_kill(task, a0, a1),
        Sysno::mkdirat => sys_mkdirat(task, a0 as _, a1 as _, a2),
        Sysno::mmap => sys_mmap(task, a0, a1, a2, a3, a4 as _, a5),
        Sysno::mount => sys_mount(task, a0 as _, a1 as _, a2 as _, a3 as _, a4),
        Sysno::munmap => sys_munmap(task, a0, a1),
        Sysno::nanosleep => sys_nanosleep(task, a0 as _, a1 as _).await,
        Sysno::openat => sys_openat(task, a0 as _, a1 as _, a2 as _, a3),
        Sysno::pipe2 => sys_pipe2(task, a0 as _, a1 as _),
        Sysno::read => sys_read(task, a0, a1 as _, a2).await,
        Sysno::readv => sys_readv(task, a0, a1 as _, a2).await,
        Sysno::setitimer => sys_setitimer(task, a0, a1 as _, a2 as _),
        Sysno::pread64 => sys_pread64(task, a0, a1 as _, a2, a3),
        Sysno::write => sys_write(task, a0, a1 as _, a2),
        Sysno::writev => sys_writev(task, a0, a1 as _, a2),
        Sysno::pwrite64 => sys_pwrite64(task, a0, a1 as _, a2, a3),
        Sysno::renameat => sys_renameat2(
            task,
            a0 as _,
            a1 as _,
            a2 as _,
            a3 as _,
            OpenFlags::RDWR.bits(),
        ),
        Sysno::sendfile => sys_sendfile(task, a0, a1, a2, a3),
        Sysno::shmget => sys_shmget(task, a0 as _, a1 as _, a2),
        Sysno::shmat => sys_shmat(task, a0, a1, a2),
        Sysno::shmctl => sys_shmctl(task, a0 as _, a1 as _, a2 as _),
        Sysno::ppoll => sys_ppoll(task, a0 as _, a1 as _, a2 as _, a3).await,
        Sysno::pselect6 => sys_pselect(task, a0, a1 as _, a2 as _, a3 as _, a4 as _, a5).await,
        Sysno::rt_sigaction => sys_sigaction(task, a0, a1 as _, a2 as _),
        Sysno::rt_sigprocmask => sys_sigprocmask(task, a0 as _, a1 as _, a2 as _),
        Sysno::rt_sigreturn => sys_sigreturn(task, ctx),
        Sysno::rt_sigtimedwait => sys_sigtimedwait(task),
        Sysno::tkill => sys_tkill(task, a0, a1),
        Sysno::sched_yield => sys_sched_yield(task),
        Sysno::set_tid_address => sys_set_tid_addr(task, a0),
        Sysno::umount2 => sys_umount(task, a0 as _, a1 as _),
        Sysno::uname => sys_uname(task, a0 as _),
        Sysno::unlinkat => sys_unlinkat(task, a0 as _, a1 as _, a2 as _),
        Sysno::utimensat => sys_utimensat(task, a0 as _, a1 as _, a2 as _, a3),
        Sysno::wait4 => sys_wait4(task, ctx, a0 as _, a1 as _, a2 as _).await,
        Sysno::prlimit64 => sys_prlimit64(task, a0, a1, a2 as _, a3 as _),
        Sysno::mprotect | Sysno::msync | Sysno::sync | Sysno::fsync => Ok(0),
        Sysno::get_robust_list => {
            log::warn!("get_robust_list not implementation");
            Ok(0)
        }
        Sysno::getuid | Sysno::getgid | Sysno::geteuid | Sysno::getegid => Ok(0),
        _ => Err(Errno::EPERM),
    }
}
