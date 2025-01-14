use crate::{child_test::TASK_MAP, syscall::SysResult};

pub(crate) fn sys_exit(badge: u64, exit_code: i32) -> SysResult {
    let mut task_map = TASK_MAP.lock();
    let task = task_map.get_mut(&badge).unwrap();
    task.exit = Some(exit_code);
    task.tcb.tcb_suspend().unwrap();
    Ok(0)
}

pub(crate) fn sys_exit_group(badge: u64, exit_code: i32) -> SysResult {
    let mut task_map = TASK_MAP.lock();
    let task = task_map.get_mut(&badge).unwrap();
    task.exit = Some(exit_code);
    task.tcb.tcb_suspend().unwrap();
    Ok(0)
}

pub(crate) fn sys_sched_yield() -> SysResult {
    sel4::r#yield();
    Ok(0)
}
