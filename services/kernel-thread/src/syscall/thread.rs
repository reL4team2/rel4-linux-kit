//! 线程相关系统调用
//!
//!

use common::page::PhysPage;
use crate_consts::PAGE_SIZE;
use sel4::UserContext;
use zerocopy::IntoBytes;

use crate::{
    child_test::TASK_MAP,
    consts::task::PAGE_COPY_TEMP,
    task::Sel4Task,
    utils::{obj::alloc_page, page::map_page_self},
};

use super::{SysResult, types::thread::CloneFlags};

/// 获取进程 id
#[inline]
pub fn sys_getpid(task: &Sel4Task) -> SysResult {
    Ok(task.pid)
}

/// 获取父进程 id
#[inline]
pub fn sys_getppid(task: &Sel4Task) -> SysResult {
    Ok(task.ppid)
}

#[inline]
pub(super) fn sys_set_tid_addr(task: &mut Sel4Task, addr: usize) -> SysResult {
    task.clear_child_tid = Some(addr);
    Ok(task.id)
}

#[inline]
pub(super) fn sys_exit(task: &mut Sel4Task, exit_code: i32) -> SysResult {
    debug!("sys_exit @ exit_code: {} ", exit_code);
    task.exit = Some(exit_code);
    Ok(0)
}

#[inline]
pub(super) fn sys_wait4(
    task: &Sel4Task,
    ctx: &mut UserContext,
    pid: isize,
    status: *const i32,
    option: usize,
) -> SysResult {
    log::warn!("wait for {} ptr: {:p} option: {}", pid, status, option);
    if option != 0 {
        panic!("option != 0 is not supported");
    }
    let mut task_map = TASK_MAP.lock();
    let finded = task_map
        .iter()
        .find(|(_, target)| target.exit.is_some() && target.ppid == task.pid);

    if finded.is_none() {
        *ctx.pc_mut() -= 4;
        return Ok(pid as _);
    }
    let (idx, exit_code) = finded.map(|x| (*x.0, x.1.exit.unwrap())).unwrap();
    task.write_bytes(status as _, (exit_code << 8).as_bytes());

    task_map.remove(&idx);
    Ok(idx as _)
}

#[inline]
pub(super) fn sys_clone(
    task: &Sel4Task,
    flags: u32,       // 复制 标志位
    stack: usize,     // 指定新的栈，可以为 0, 0 不处理
    ptid: *const u32, // 父线程 id
    tls: usize,       // TLS线程本地存储描述符
    ctid: *const u32, // 子线程 id
) -> SysResult {
    let signal = flags & 0xff;
    let flags = CloneFlags::from_bits_truncate(flags);
    if !flags.is_empty() {
        panic!("Custom Clone is not supported");
    }
    log::debug!(
        "flags: {:?} signal: {} stack: {:#x}, ptid: {:p}  tls: {:#x}, ctid: {:#p}",
        flags,
        signal,
        stack,
        ptid,
        tls,
        ctid
    );

    let mut new_task = Sel4Task::new().unwrap();
    let new_task_id = new_task.id;
    new_task.signal.exit_sig = signal;
    new_task.ppid = task.pid;
    let mut regs = task.tcb.tcb_read_all_registers(false).unwrap();
    *regs.c_param_mut(0) = 0;
    *regs.pc_mut() += 4;
    if stack != 0 {
        *regs.sp_mut() = stack as _;
    }
    new_task.init_tcb().unwrap();

    // 复制文件表
    {
        let mut new_ft = new_task.file.file_ds.lock();
        let old_ft = task.file.file_ds.lock();

        for idx in 0..=512 {
            if let Some(fd) = old_ft.get(idx) {
                let _ = new_ft.add_at(idx, fd.clone());
            }
        }
        new_task.file.work_dir = task.file.work_dir.clone();
    }

    // 复制映射的地址
    {
        let old_mem_info = task.mem.lock();
        for (vaddr, page) in old_mem_info.mapped_page.iter() {
            let new_page = alloc_page();
            map_page_self(PAGE_COPY_TEMP, new_page);
            unsafe {
                let mut page_locker = page.lock();
                (PAGE_COPY_TEMP as *mut u128).copy_from_nonoverlapping(
                    page_locker.as_mut_ptr() as *mut _,
                    PAGE_SIZE / size_of::<u128>(),
                );
                drop(page_locker)
            }
            new_page.frame_unmap().unwrap();
            new_task.map_page(*vaddr, PhysPage::new(new_page));
        }
    }
    new_task
        .tcb
        .tcb_write_all_registers(true, &mut regs)
        .unwrap();
    TASK_MAP.lock().insert(new_task_id as _, new_task);

    Ok(new_task_id)
}
