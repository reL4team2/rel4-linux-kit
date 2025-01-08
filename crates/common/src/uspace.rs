//! It defines all kinds of configuration about user space.

use core::net::{Ipv4Addr, SocketAddr};

pub const USPACE_HEAP_BASE: usize = 0x1_0000_0000;
pub const USPACE_HEAP_SIZE: usize = 0x10_0000;

/// The highest address of the user space stack
pub const USPACE_STACK_TOP: usize = 0x2_0000_0000;
/// The maximum size of the user space stack
pub const USPACE_STACK_SIZE: usize = 0x1_0000;

/// The file descriptor for stdin
pub const STDIN_FD: i32 = 0;
/// The file descriptor for stdout
pub const STDOUT_FD: i32 = 1;
/// The file descriptor for stderr
pub const STDERR_FD: i32 = 2;

/// The lowest address of the user space
pub const USPACE_BASE: usize = 0x1000;

/// A void pointer in C
pub type CVoidPtr = usize;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct LibcSocketAddr {
    pub sa_family: u16,
    pub sa_data: [u8; 14usize],
}

impl From<SocketAddr> for LibcSocketAddr {
    fn from(value: SocketAddr) -> Self {
        let mut addr = LibcSocketAddr::default();
        // FIXME: It default use AF_INET domain. And it use ipv4 address.
        addr.sa_family = 2;
        let ip = value.ip();
        match ip {
            core::net::IpAddr::V4(ip) => {
                let ip = ip.octets();
                for i in 0..4 {
                    addr.sa_data[i + 2] = ip[i];
                }
            }
            core::net::IpAddr::V6(ip) => {
                let ip = ip.octets();
                for i in 0..12 {
                    addr.sa_data[i + 2] = ip[i];
                }
            }
        }
        addr.sa_data[0] = (value.port() >> 8) as u8;
        addr.sa_data[1] = (value.port() & 0xff) as u8;
        addr
    }
}

impl Into<SocketAddr> for LibcSocketAddr {
    fn into(self) -> SocketAddr {
        // FIXME: It default use AF_INET domain. And it use ipv4 address.
        let data = self.sa_data;
        let port = u16::from_be_bytes([data[0], data[1]]);

        SocketAddr::new(
            core::net::IpAddr::V4(Ipv4Addr::new(data[2], data[3], data[4], data[5])),
            port,
        )
    }
}

bitflags::bitflags! {
    /// 用于 sys_clone 的选项
    #[derive(Debug, Clone, Copy)]
    pub struct CloneFlags: i32 {
        /// .
        const CLONE_NEWTIME = 1 << 7;
        /// Share the same VM  between processes
        const CLONE_VM = 1 << 8;
        /// Share the same fs info between processes
        const CLONE_FS = 1 << 9;
        /// Share open files between processes
        const CLONE_FILES = 1 << 10;
        /// Share signal handlers between processes
        const CLONE_SIGHAND = 1 << 11;
        /// Place a pidfd in the parent's pidfd
        const CLONE_PIDFD = 1 << 12;
        /// Continue tracing in the chil
        const CLONE_PTRACE = 1 << 13;
        /// Suspends the parent until the child wakes up
        const CLONE_VFORK = 1 << 14;
        /// Current process shares the same parent as the cloner
        const CLONE_PARENT = 1 << 15;
        /// Add to the same thread group
        const CLONE_THREAD = 1 << 16;
        /// Create a new namespace
        const CLONE_NEWNS = 1 << 17;
        /// Share SVID SEM_UNDO semantics
        const CLONE_SYSVSEM = 1 << 18;
        /// Set TLS info
        const CLONE_SETTLS = 1 << 19;
        /// Store TID in userlevel buffer in the parent before MM copy
        const CLONE_PARENT_SETTID = 1 << 20;
        /// Register exit futex and memory location to clear
        const CLONE_CHILD_CLEARTID = 1 << 21;
        /// Create clone detached
        const CLONE_DETACHED = 1 << 22;
        /// The tracing process can't force CLONE_PTRACE on this clone.
        const CLONE_UNTRACED = 1 << 23;
        /// Store TID in userlevel buffer in the child
        const CLONE_CHILD_SETTID = 1 << 24;
        /// New pid namespace.
        const CLONE_NEWPID = 1 << 29;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CloneArgs {
    pub stack: *const u8,
    pub flags: i32,
    pub parent_tid: *const u8,
    pub child_tid: *const u8,
    pub tls: *const u8,
    pub init_fn: *const u8,
    pub init_argv: *const u8,
}

impl Default for CloneArgs {
    fn default() -> Self {
        Self {
            stack: core::ptr::null(),
            flags: 0,
            parent_tid: core::ptr::null(),
            child_tid: core::ptr::null(),
            tls: core::ptr::null(),
            init_fn: core::ptr::null(),
            init_argv: core::ptr::null(),
        }
    }
}
