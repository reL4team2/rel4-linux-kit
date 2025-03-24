//! 文件相关系统调用类型
//!
//!
use num_enum::TryFromPrimitive;

/// Fcntl 相关的指令
#[repr(u32)]
#[derive(Debug, Clone, PartialEq, TryFromPrimitive)]
pub enum FcntlCmd {
    /// dup
    DUPFD = 0,
    /// get close_on_exec
    GETFD = 1,
    /// set/clear close_on_exec
    SETFD = 2,
    /// get file->f_flags
    GETFL = 3,
    /// set file->f_flags
    SETFL = 4,
    /// Get record locking info.
    GETLK = 5,
    /// Set record locking info (non-blocking).
    SETLK = 6,
    /// Set record locking info (blocking).
    SETLKW = 7,
    /// like F_DUPFD, but additionally set the close-on-exec flag
    DUPFDCLOEXEC = 0x406,
}
