use sel4::{cap::Endpoint, debug_println};
use syscalls::{Errno, Sysno};
mod fs;
mod mm;
mod net;
mod thread;

pub type SysResult = Result<usize, Errno>;

pub fn handle_ipc_call(
    badge: u64,
    sys_id: usize,
    args: [usize; 6],
    fault_ep: Endpoint,
) -> Result<usize, Errno> {
    let sys_no = Sysno::new(sys_id).ok_or(Errno::EINVAL)?;
    debug_println!("[KernelThread] Syscall: {:?}", sys_no);
    match sys_no {
        Sysno::write => fs::sys_write(badge, args[0] as _, args[1] as _, args[2] as _),
        Sysno::brk => mm::sys_brk(badge, args[0] as _),
        Sysno::mmap => mm::sys_mmap(
            badge,
            args[0] as _,
            args[1] as _,
            args[2] as _,
            args[3] as _,
            args[4] as _,
            args[5] as _,
        ),
        Sysno::munmap => mm::sys_unmap(badge, args[0] as _, args[1] as _),
        Sysno::exit => thread::sys_exit(badge, args[0] as _),
        Sysno::exit_group => thread::sys_exit_group(badge, args[0] as _),
        Sysno::getpid => thread::sys_getpid(badge),
        Sysno::execve => {
            thread::sys_exec(badge, fault_ep, args[0] as _, args[1] as _, args[2] as _)
        }
        Sysno::clone => thread::sys_clone(badge, fault_ep, args[0] as _, args[1] as _),
        Sysno::gettid => thread::sys_gettid(badge as _),
        Sysno::sched_yield => thread::sys_sched_yield(),
        Sysno::getppid => thread::sys_getppid(badge),
        Sysno::set_tid_address => thread::sys_set_tid_address(badge, args[0] as _),
        Sysno::getuid => thread::sys_getuid(badge),
        Sysno::geteuid => thread::sys_geteuid(badge),

        Sysno::socket => net::sys_socket(badge, args[0] as _, args[1] as _, args[2] as _),
        Sysno::accept => net::sys_accept(badge, args[0] as _, args[1] as _, args[2] as _),
        Sysno::bind => net::sys_bind(badge, args[0] as _, args[1] as _, args[2] as _),
        Sysno::connect => net::sys_connect(badge, args[0] as _, args[1] as _, args[2] as _),
        Sysno::listen => net::sys_listen(badge, args[0] as _),
        Sysno::sendto => net::sys_sendto(
            badge,
            args[0] as _,
            args[1] as _,
            args[2] as _,
            args[3] as _,
            args[4] as _,
            args[5] as _,
        ),
        Sysno::recvfrom => net::sys_recvfrom(
            badge,
            args[0] as _,
            args[1] as _,
            args[2] as _,
            args[3] as _,
            args[4] as _,
            args[5] as _,
        ),
        Sysno::shutdown => net::sys_shutdown(badge, args[0] as _, args[1] as _),
        _ => Err(Errno::ENOSYS),
    }
}
