//! 进程控制块和进程信息

use core::time::Duration;

use libc_core::time::ITimerVal;
use spin::Mutex;

/// 进程控制块
pub struct ProcessControlBlock {
    // /// 进程 ID
    // pub pid: usize,
    // /// 父进程 ID
    // pub ppid: usize,
    /// 定时器信息
    pub itimer: Mutex<[ProcessTimer; 3]>,
}

#[derive(Debug, Clone, Default, zerocopy::KnownLayout)]
pub struct ProcessTimer {
    /// 定时器信息
    pub timer: ITimerVal,
    /// 下一个定时器信息
    pub next: Duration,
}

impl ProcessControlBlock {
    pub fn new() -> Self {
        Self {
            itimer: Mutex::new([
                ProcessTimer::default(),
                ProcessTimer::default(),
                ProcessTimer::default(),
            ]),
        }
    }
}
