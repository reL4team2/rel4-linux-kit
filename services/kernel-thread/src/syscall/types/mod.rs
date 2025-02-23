//! SysCall 使用到的类型定义
//!
//!
pub mod signal;
pub mod sys;
use zerocopy::{FromBytes, Immutable};

#[repr(C)]
#[derive(Clone, FromBytes, Immutable)]
pub(super) struct IoVec {
    pub base: usize,
    pub len: usize,
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
