//! 线程相关类型定义
//!
//!

bitflags::bitflags! {
    /// `CloneFlags` 用于表示 `sys_clone` 的 `flags` 参数。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CloneFlags: u32 {
        /// 共享地址空间（类似 `pthread`）
        const CLONE_VM           = 1 << 8;
        /// 共享文件系统信息（工作目录、根目录、挂载点）
        const CLONE_FS           = 1 << 9;
        /// 共享文件描述符表
        const CLONE_FILES        = 1 << 10;
        /// 共享信号处理程序
        const CLONE_SIGHAND      = 1 << 11;
        /// 类似 `vfork()`，父进程会挂起直到子进程调用 `exec()` 或 `_exit()`
        const CLONE_VFORK        = 1 << 14;
        /// 让新进程的 `parent` 设为父进程的 `parent`
        const CLONE_PARENT       = 1 << 15;
        /// 共享信号掩码
        const CLONE_THREAD       = 1 << 16;
        /// 新建挂载命名空间
        const CLONE_NEWNS        = 1 << 17;
        /// 共享 System V IPC 信号量
        const CLONE_SYSVSEM      = 1 << 18;
        /// 为子进程设置 TLS（线程本地存储）
        const CLONE_SETTLS       = 1 << 19;
        /// 在子进程的用户空间存储 `tid`
        const CLONE_PARENT_SETTID = 1 << 20;
        /// 进程退出时清除 `tid`
        const CLONE_CHILD_CLEARTID = 1 << 21;
        /// 在子进程的用户空间存储 `tid`
        const CLONE_CHILD_SETTID = 1 << 24;
        /// 新建 UTS 命名空间（主机名、域名隔离）
        const CLONE_NEWUTS       = 1 << 26;
        /// 新建用户命名空间（隔离用户 ID 和组 ID）
        const CLONE_NEWUSER      = 1 << 28;
        /// 新建 PID 命名空间（使子进程的 PID 重新从 1 计算）
        const CLONE_NEWPID       = 1 << 29;
        /// 新建网络命名空间（隔离网络设备、IP）
        const CLONE_NEWNET       = 1 << 30;

    }

    /// `WaitOption` 用于表示 `sys_wait4` 的 `option` 参数
    #[derive(Debug, Clone, Copy)]
    pub struct WaitOption: u32 {
        /// 如果没有子进程退出，wait4 立即返回 0，而不会阻塞等待。
        const WHOHANG    =     1 << 0;
        /// 让 wait4 也返回因 SIGSTOP (如 CTRL+Z) 暂停的子进程信息。
        /// 默认情况下，wait4 只会返回已退出的子进程。
        const WUNTRACED  =     1 << 1;
    }

}
