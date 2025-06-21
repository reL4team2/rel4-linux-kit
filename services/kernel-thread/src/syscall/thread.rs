//! 线程相关系统调用
//!
//!

use core::pin::pin;

use alloc::{string::String, sync::Arc, vec::Vec};
use common::{config::PAGE_SIZE, page::PhysPage, slot::alloc_slot};
use flatten_objects::FlattenObjects;
use fs::file::File;
use futures::future::{Either, select};
use libc_core::{
    fcntl::{AT_FDCWD, OpenFlags},
    futex::FutexFlags,
    sched::{CloneFlags, WaitOption},
    signal::SignalNum,
    time::ITimerVal,
    types::TimeSpec,
};
use object::Object;
use sel4::{CapRights, UserContext};
use sel4_kit::{arch::current_time, slot_manager::LeafSlot};
use spin::mutex::Mutex;
use syscalls::Errno;
use zerocopy::{FromBytes, IntoBytes};

use crate::{
    child_test::{ArcTask, TASK_MAP, WaitAnyChild, WaitPid, futex_requeue, futex_wake, wait_futex},
    consts::task::{DEF_STACK_TOP, PAGE_COPY_TEMP},
    task::Sel4Task,
    timer::{set_process_timer, wait_time},
    utils::page::map_page_self,
};

use super::SysResult;

/// 获取进程 id
#[inline]
pub fn sys_getpid(task: &Sel4Task) -> SysResult {
    Ok(task.pid)
}

/// 获取线程 ID
#[inline]
pub fn sys_gettid(task: &Sel4Task) -> SysResult {
    Ok(task.pid)
}

/// 获取父进程 id
#[inline]
pub fn sys_getppid(task: &Sel4Task) -> SysResult {
    Ok(task.ppid)
}

#[inline]
pub(super) fn sys_set_tid_addr(task: &Sel4Task, addr: usize) -> SysResult {
    *task.clear_child_tid.lock() = addr;
    Ok(task.tid)
}

#[inline]
pub(super) fn sys_exit(task: &Sel4Task, exit_code: u32) -> SysResult {
    debug!("sys_exit @ exit_code: {} ", exit_code);
    task.exit_with(exit_code << 8);
    Ok(0)
}

#[inline]
pub(super) async fn sys_wait4(
    task: &Sel4Task,
    ctx: &mut UserContext,
    pid: isize,
    status: *const i32,
    option: u32,
) -> SysResult {
    log::warn!("wait for {} ptr: {:p} option: {}", pid, status, option);
    let options = WaitOption::from_bits_truncate(option);
    if options.contains(WaitOption::WUNTRACED) {
        panic!("option({:?}  {}) is not supported", options, option);
    }

    if !TASK_MAP.lock().iter().any(|x| x.1.ppid == task.pid) {
        return Err(Errno::ECHILD);
    }

    let finded = if pid == -1 {
        WaitAnyChild(task.pid as _, options.contains(WaitOption::WHOHANG)).await
    } else if pid > 0 {
        WaitPid(
            task.pid as _,
            pid as _,
            options.contains(WaitOption::WHOHANG),
        )
        .await
    } else {
        TASK_MAP
            .lock()
            .iter()
            .find(|(_, target)| target.exit.lock().is_some() && target.ppid == task.pid)
            .map(|(&tid, task)| (tid, task.exit.lock().unwrap()))
    };

    if finded.is_none() {
        if options.contains(WaitOption::WHOHANG) {
            return Ok(0);
        }
        *ctx.pc_mut() -= 4;
        return Ok(pid as _);
    }
    let (idx, exit_code) = finded.map(|x| (x.0, x.1)).unwrap();
    task.write_bytes(status as _, exit_code.as_bytes());

    TASK_MAP.lock().remove(&idx);
    Ok(idx as _)
}

#[inline]
pub(super) async fn sys_clone(
    task: &Sel4Task,
    flags: u32,       // 复制 标志位
    stack: usize,     // 指定新的栈，可以为 0, 0 不处理
    ptid: *const u32, // 父线程 id
    tls: usize,       // TLS线程本地存储描述符
    ctid: *const u32, // 子线程 id
) -> SysResult {
    let signal = flags & 0xff;
    let flags = CloneFlags::from_bits_truncate(flags);
    log::debug!(
        "flags: {:?} signal: {} stack: {:#x}, ptid: {:p}  tls: {:#x}, ctid: {:#p}",
        flags,
        signal,
        stack,
        ptid,
        tls,
        ctid
    );
    let mut new_task = if flags.bits() > 0xff {
        if flags.contains(CloneFlags::CLONE_THREAD) {
            task.create_thread().unwrap()
        } else if !flags.contains(CloneFlags::CLONE_VM) {
            Sel4Task::new().unwrap()
        } else {
            log::error!(
                "flags: {:?} signal: {} stack: {:#x}, ptid: {:p}  tls: {:#x}, ctid: {:#p}",
                flags,
                signal,
                stack,
                ptid,
                tls,
                ctid
            );
            panic!("Custom Clone is not supported");
        }
    } else {
        Sel4Task::new().unwrap()
    };
    let new_task_id = new_task.tid;
    new_task.signal.lock().exit_sig = SignalNum::from_num(signal as _);
    new_task.signal.lock().mask = task.signal.lock().mask;
    new_task.ppid = task.pid;

    let mut regs = task.tcb.tcb_read_all_registers(true).unwrap();
    *regs.c_param_mut(0) = 0;
    *regs.pc_mut() += 4;
    if stack != 0 {
        *regs.sp_mut() = stack as _;
    }
    new_task.init_tcb().unwrap();

    if flags.contains(CloneFlags::CLONE_SETTLS) {
        regs.inner_mut().tpidr_el0 = tls as _;
        regs.inner_mut().tpidrro_el0 = tls as _;
    }

    if flags.contains(CloneFlags::CLONE_PARENT_SETTID) {
        task.write_bytes(ptid as _, new_task_id.as_bytes());
    }

    if flags.contains(CloneFlags::CLONE_CHILD_SETTID) {
        task.write_bytes(ctid as _, new_task_id.as_bytes());
    }

    // 复制文件表
    if !flags.contains(CloneFlags::CLONE_FILES) {
        new_task.file.file_ds = Arc::new(Mutex::new(FlattenObjects::new()));
        let mut new_ft = new_task.file.file_ds.lock();
        let old_ft = task.file.file_ds.lock();

        for idx in 0..=512 {
            if let Some(fd) = old_ft.get(idx) {
                let _ = new_ft.add_at(idx, fd.clone());
            }
        }
    }
    if flags.contains(CloneFlags::CLONE_FS) {
        new_task.file.work_dir = task.file.work_dir.clone();
    }

    if flags.contains(CloneFlags::CLONE_SIGHAND) {
        new_task.signal.lock().actions = task.signal.lock().actions.clone();
    }

    let clear_child_tid = if flags.contains(CloneFlags::CLONE_CHILD_CLEARTID) {
        ctid as usize
    } else {
        0
    };
    *new_task.clear_child_tid.lock() = clear_child_tid;

    // 复制映射的地址
    if !flags.contains(CloneFlags::CLONE_VM) {
        let old_mem_info = task.mem.lock();
        for (vaddr, page) in old_mem_info.mapped_page.iter() {
            // 不复制共享的内存
            if task.shm.lock().iter().any(|x| x.contains(*vaddr)) {
                continue;
            }
            let new_page = new_task.capset.lock().alloc_page();
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
        // 处理 Share Memory
        task.shm.lock().iter().for_each(|maped_shared_memory| {
            new_task.shm.lock().push(maped_shared_memory.clone());
            if maped_shared_memory.start >= DEF_STACK_TOP - 16 * PAGE_SIZE {
                // 不复制栈内存
                return;
            }
            maped_shared_memory
                .mem
                .trackers
                .iter()
                .enumerate()
                .for_each(|(i, page)| {
                    let new_slot = alloc_slot();
                    new_slot
                        .copy_from(&LeafSlot::from_cap(*page), CapRights::all())
                        .unwrap();
                    new_task.map_page(
                        maped_shared_memory.start + i * PAGE_SIZE,
                        PhysPage::new(new_slot.cap()),
                    );
                });
        });
    }

    new_task
        .tcb
        .tcb_write_all_registers(true, &mut regs)
        .unwrap();
    TASK_MAP.lock().insert(new_task_id as _, Arc::new(new_task));
    // wait_time(current_time() + Duration::new(0, 1000000), task.tid).await?;
    Ok(new_task_id)
}

pub(super) fn sys_execve(
    task: &Sel4Task,
    ctx: &mut UserContext,
    path: *const u8,
    args: *const *const u8,
    envp: *const *const u8,
) -> SysResult {
    let path = task.fd_resolve(AT_FDCWD, path)?;
    let argsp = if !args.is_null() {
        task.read_vec(args as _).ok_or(Errno::EINVAL)?
    } else {
        Vec::new()
    };
    let envpp = if !envp.is_null() {
        task.read_vec(envp as _).ok_or(Errno::EINVAL)?
    } else {
        Vec::new()
    };
    let args = argsp
        .iter()
        .map(|x| task.read_cstr(*x).ok_or(Errno::EINVAL))
        .map(|x| x.map(|x| String::from_utf8(x).unwrap()))
        .collect::<Result<Vec<_>, Errno>>()?;
    let _envp = envpp
        .iter()
        .map(|x| task.read_cstr(*x).ok_or(Errno::EINVAL))
        .map(|x| x.map(|x| String::from_utf8(x).unwrap()))
        .collect::<Result<Vec<_>, Errno>>()?;

    let file = File::open(path, OpenFlags::RDONLY)?;

    task.clear_maped();

    let mut file_data = vec![0u8; file.file_size().unwrap()];
    file.read(&mut file_data)?;
    // let file_data = file.read_all().();
    let file = object::File::parse(file_data.as_slice()).expect("can't load elf file");
    task.load_elf(&file);

    // 填充初始化信息
    task.info.lock().entry = file.entry() as _;
    task.info.lock().args = args;

    // 映射栈内存并填充初始化信息
    task.map_region(DEF_STACK_TOP - 16 * PAGE_SIZE, DEF_STACK_TOP);
    let sp_ptr = task.init_stack();

    // 写入线程的寄存器信息
    {
        *ctx = sel4::UserContext::default();
        *ctx.pc_mut() = (task.info.lock().entry - 4) as _;
        *ctx.sp_mut() = sp_ptr as _;
    }

    Ok(0)
}

#[inline]
pub(super) fn sys_sched_yield(_task: &Sel4Task) -> SysResult {
    Ok(0)
}

pub(super) async fn sys_futex(
    task: ArcTask,
    uaddr_ptr: *mut i32,
    op: usize,
    value: usize,
    value2: usize,
    uaddr2: usize,
    _value3: usize,
) -> SysResult {
    let op = if op >= 0x80 { op - 0x80 } else { op };
    let uaddr = task.read_ins(uaddr_ptr as _).ok_or(Errno::EINVAL)?;
    let flags = FutexFlags::try_from(op).map_err(|_| Errno::EINVAL)?;
    debug!(
        "task {} sys_futex @ op uaddr: {:p} flags: {:?} value: {:#x} value2: {:#x}",
        task.tid, uaddr_ptr, flags, value as u32, value2
    );

    match flags {
        FutexFlags::Wait => {
            if uaddr == value as _ {
                let wait_func = wait_futex(task.clone(), uaddr_ptr as _);
                if value2 != 0 {
                    let timespec_bytes = task.read_bytes(value2, size_of::<TimeSpec>()).unwrap();
                    let timespec = TimeSpec::ref_from_bytes(&timespec_bytes).unwrap();
                    let next = current_time() + (*timespec).into();
                    let timeout_func = wait_time(next, task.tid);
                    match select(pin!(wait_func), pin!(timeout_func)).await {
                        Either::Left((res, _)) => res,
                        Either::Right((res, _)) => res,
                    }
                } else {
                    wait_func.await
                }
            } else {
                Err(Errno::EAGAIN)
            }
        }
        FutexFlags::Wake => {
            let futex_table = task.futex_table.clone();
            let count = futex_wake(futex_table, uaddr_ptr as _, value);
            Ok(count)
        }
        FutexFlags::Requeue => {
            let futex_table = task.futex_table.clone();
            Ok(futex_requeue(
                futex_table,
                uaddr_ptr as _,
                value,
                uaddr2,
                value2,
            ))
        }
        _ => Err(Errno::EPERM),
    }
}

pub(super) fn sys_tkill(task: &Sel4Task, tid: usize, signum: usize) -> SysResult {
    debug!("sys_tkill @ tid: {}, signum: {}", tid, signum);
    let target_signal = SignalNum::from_num(signum).ok_or(Errno::EINVAL)?;
    let mut task_map = TASK_MAP.lock();
    let target = if tid == task.tid {
        task
    } else {
        task_map
            .iter_mut()
            .find(|x| *x.0 == tid as _)
            .map(|(_, task)| task)
            .ok_or(Errno::ESRCH)?
    };

    target.add_signal(target_signal, task.tid);
    Ok(0)
}

pub(super) fn sys_setitimer(
    task: &Sel4Task,
    which: usize,
    times_ptr: *mut ITimerVal,
    old_timer_ptr: *mut ITimerVal,
) -> SysResult {
    debug!(
        "[task {}] sys_setitimer @ which: {} times_ptr: {:p} old_timer_ptr: {:p}",
        task.tid, which, times_ptr, old_timer_ptr
    );

    if which == 0 {
        let pcb = task.pcb.clone();
        if !old_timer_ptr.is_null() {
            task.write_bytes(old_timer_ptr as _, pcb.itimer.lock()[0].timer.as_bytes());
        }
        if !times_ptr.is_null() {
            let current_timval = current_time();
            let new_timer_bytes = task
                .read_bytes(times_ptr as _, size_of::<ITimerVal>())
                .ok_or(Errno::EINVAL)?;

            let new_timer = ITimerVal::ref_from_bytes(&new_timer_bytes).unwrap();
            pcb.itimer.lock()[0].timer = new_timer.clone();
            pcb.itimer.lock()[0].next = current_timval + new_timer.value.into();

            set_process_timer(task.pid, pcb.itimer.lock()[0].next);
        }
        Ok(0)
    } else {
        log::error!("not support case for setitimer");
        Err(Errno::EPERM)
    }
}
