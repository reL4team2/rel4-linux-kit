//! seL4 平台接口定义

#![no_std]
#![deny(warnings)]
#![deny(missing_docs)]

use axplat_macros::def_plat_interface;

#[doc(hidden)]
pub mod __priv {
    pub use const_str::equal as const_str_eq;
    pub use crate_interface::{call_interface, def_interface};
}

/// 任务接口
#[def_plat_interface]
pub trait Sel4TaskIf {
    /// Switches to the given seL4 task.
    ///
    /// It returns the previous task's ID.
    fn switch_task(prev_task: usize, next_task: usize) -> usize;

    /// Creates a new seL4 task with the given parameters.
    ///
    /// It returns the created task's ID.
    fn create_task(
        tid: usize,
        entry_point: usize,
        stack_top: usize,
        priority: usize,
        cpu_id: usize,
    ) -> usize;

    /// Destroys the seL4 task with the given ID.
    fn destroy_task(task_id: usize);

    /// Migrates the seL4 task with the given ID to the target CPU.
    fn migrate_task(task_id: usize, target_cpu_id: usize);

    /// Starts the seL4 task with the given ID.
    fn start_task(task_id: usize);

    /// Stops the seL4 task with the given ID.
    fn stop_task(task_id: usize);

    /// Checks if the current task is the initial task.
    fn is_init_task() -> bool;

    /// Get Current Sel4 Task ID
    fn sel4_task_id() -> usize;
}

/// 事件处理接口
#[def_plat_interface]
pub trait Sel4EventIf {
    /// 事件处理函数
    fn handler(cpu_id: usize) -> !;
}

/// 中断相关接口
#[cfg(feature = "irq")]
#[def_plat_interface]
pub trait Sel4IrqIf {
    /// Disables IRQs.
    fn disable_irqs();

    /// Enables IRQs.
    fn enable_irqs();

    /// Checks if IRQs are enabled.
    fn irqs_enabled() -> bool;
}
