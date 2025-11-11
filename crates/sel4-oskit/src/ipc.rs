//! 定义了默认会使用到的 ipc 接口
//! 比如创建、切换、退出、迁移任务等等

use common_macros::generate_ipc_send;
use num_enum::{IntoPrimitive, TryFromPrimitive};

/// 定义服务事件枚举
#[derive(Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u64)]
pub enum ServiceEvent {
    /// 创建任务
    CreateTask = 0x1000,
    /// 切换任务
    SwitchTask,
    /// 退出任务
    ExitTask,
    /// 退出系统
    ExitSystem,
    /// 迁移任务
    MigrateTask,
}

macro_rules! call_ep {
    ($msg:expr) => {
        $crate::config::DEFAULT_PARENT_EP.call($msg)
    };
}

/// 创建一个任务
#[generate_ipc_send(label = ServiceEvent::CreateTask)]
pub fn create_task(tid: usize, entry: usize, kstack: usize, tls: usize, affinity: usize) -> usize {}

/// 切换任务
#[generate_ipc_send(label = ServiceEvent::SwitchTask)]
pub fn switch_task(prev_task: usize, next_task: usize) -> usize {}

/// 退出任务
#[generate_ipc_send(label = ServiceEvent::ExitTask)]
pub fn exit_task(task: usize) -> usize {}

/// 退出系统
#[generate_ipc_send(label = ServiceEvent::ExitSystem)]
pub fn exit_system() -> usize {}

/// 迁移任务
#[generate_ipc_send(label = ServiceEvent::MigrateTask)]
pub fn migrate_task(task: usize, cpu_id: usize) -> usize {}
