//! 线程相关系统调用
//!
//!

use alloc::{string::String, vec::Vec};
use common::page::PhysPage;
use config::PAGE_SIZE;
use object::Object;
use sel4::UserContext;
use syscalls::Errno;
use zerocopy::IntoBytes;

use crate::{
    child_test::TASK_MAP,
    consts::{
        fd::{DEF_OPEN_FLAGS, FD_CUR_DIR},
        task::{DEF_STACK_TOP, PAGE_COPY_TEMP},
    },
    fs::file::File,
    syscall::types::thread::WaitOption,
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
    option: u32,
) -> SysResult {
    log::warn!("wait for {} ptr: {:p} option: {}", pid, status, option);
    let options = WaitOption::from_bits_truncate(option);
    if options.contains(WaitOption::WUNTRACED) {
        panic!("option({:?}  {}) is not supported", options, option);
    }
    let mut task_map = TASK_MAP.lock();
    let finded = task_map
        .iter()
        .find(|(_, target)| target.exit.is_some() && target.ppid == task.pid);

    if finded.is_none() {
        if options.contains(WaitOption::WHOHANG) {
            return Ok(0);
        }
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

pub(super) fn sys_execve(
    task: &mut Sel4Task,
    ctx: &mut UserContext,
    path: *const u8,
    args: *const *const u8,
    envp: *const *const u8,
) -> SysResult {
    let path = task.deal_path(FD_CUR_DIR, path)?;
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

    let mut file = File::open(&path, DEF_OPEN_FLAGS)?;

    task.clear_maped();

    let file_data = file.read_all().unwrap();
    let file = object::File::parse(file_data.as_slice()).expect("can't load elf file");
    task.load_elf(&file);

    // 填充初始化信息
    task.info.entry = file.entry() as _;
    task.info.args = args;

    // 映射栈内存并填充初始化信息
    task.map_region(DEF_STACK_TOP - 16 * PAGE_SIZE, DEF_STACK_TOP);
    let sp_ptr = task.init_stack();

    // 写入线程的寄存器信息
    {
        *ctx = sel4::UserContext::default();
        *ctx.pc_mut() = (task.info.entry - 4) as _;
        *ctx.sp_mut() = sp_ptr as _;
    }

    Ok(0)
}

#[inline]
pub(super) fn sys_sched_yield(_task: &mut Sel4Task) -> SysResult {
    Ok(0)
}
