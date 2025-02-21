use core::cmp;

use alloc::string::String;
use common::{
    footprint, map_image, page::PhysPage, CloneArgs, CloneFlags, USPACE_STACK_SIZE,
    USPACE_STACK_TOP,
};
use crate_consts::{CNODE_RADIX_BITS, GRANULE_SIZE};
use object::File;
use sel4::{
    cap::Endpoint,
    cap_type::{self},
    init_thread, CNodeCapData, Cap, CapRights, VmAttributes,
};
use syscalls::Errno;
use xmas_elf::ElfFile;

use crate::{
    child_test::TASK_MAP,
    page_seat_vaddr,
    syscall::SysResult,
    task::Sel4Task,
    utils::{
        init_free_page_addr,
        obj::{alloc_page, alloc_slot, OBJ_ALLOCATOR},
        read_item, FreePagePlaceHolder,
    },
};

pub(crate) fn sys_getpid(badge: u64) -> SysResult {
    Ok(TASK_MAP.lock().get(&badge).unwrap().pid as usize)
}

pub(crate) fn sys_getppid(badge: u64) -> SysResult {
    Ok(TASK_MAP.lock().get(&badge).unwrap().pid as usize)
}

pub(crate) fn sys_getuid(badge: u64) -> SysResult {
    Ok(TASK_MAP.lock().get(&badge).unwrap().id as usize)
}

pub(crate) fn sys_geteuid(badge: u64) -> SysResult {
    Ok(TASK_MAP.lock().get(&badge).unwrap().id as usize)
}

pub(crate) fn sys_gettid(badge: usize) -> SysResult {
    Ok(badge)
}

pub(crate) fn sys_set_tid_address(badge: u64, tidptr: *mut i32) -> SysResult {
    TASK_MAP.lock().get_mut(&badge).unwrap().clear_child_tid = Some(tidptr as usize);
    Ok(badge as usize)
}

pub(crate) fn sys_exec(
    badge: u64,
    fault_ep: Endpoint,
    _path: *const u8,
    _argv: *const u8,
    _envp: *const u8,
) -> SysResult {
    let mut task_map = TASK_MAP.lock();
    let task = task_map.get_mut(&badge).unwrap();
    let args = &["busybox", "--help"];

    task.mapped_page.clear();
    task.mapped_pt.clear();

    let child_image = File::parse(CHILD_ELF).unwrap();
    let mut obj_allocator = OBJ_ALLOCATOR.lock();
    map_image(
        &mut obj_allocator,
        &mut task.mapped_page,
        task.vspace,
        footprint(&child_image),
        &child_image,
        init_thread::slot::VSPACE.cap(),
        unsafe { init_free_page_addr() },
    );
    drop(obj_allocator);

    task.info.args = args.iter().map(|x| String::from(*x)).collect();
    task.map_region(USPACE_STACK_TOP - USPACE_STACK_SIZE, USPACE_STACK_TOP);
    let sp_ptr = task.init_stack();

    let ipc_buffer_cap = PhysPage::new(alloc_page());
    let max = file
        .section_iter()
        .fold(0, |acc, x| cmp::max(acc, x.address() + x.size()));

    let ipc_buffer_addr = (max + 4096 - 1) / 4096 * 4096;

    task.map_page(ipc_buffer_addr as _, ipc_buffer_cap);

    // Configure the child task
    task.tcb
        .tcb_configure(
            fault_ep.cptr(),
            task.cnode,
            CNodeCapData::new(0, sel4::WORD_SIZE - 12),
            task.vspace,
            ipc_buffer_addr,
            ipc_buffer_cap.cap(),
        )
        .unwrap();

    task.tcb
        .tcb_set_sched_params(init_thread::slot::TCB.cap(), 0, 255)
        .unwrap();

    let mut user_context = sel4::UserContext::default();

    // Set child task's context
    *user_context.pc_mut() = file.header.pt2.entry_point();
    *user_context.sp_mut() = sp_ptr as _;
    *user_context.gpr_mut(1) = ipc_buffer_addr;
    // Get TSS section address.
    user_context.inner_mut().tpidr_el0 = file
        .find_section_by_name(".tbss")
        .map_or(0, |tls| tls.address());

    task.tcb
        .tcb_write_all_registers(false, &mut user_context)
        .unwrap();

    task.tcb.debug_name(b"before name");

    task.exit = Some(0);

    task.tcb.tcb_resume().unwrap();
    Ok(0)
}

pub(crate) fn sys_clone(
    badge: u64,
    fault_ep: Endpoint,
    clone_args: *const CloneArgs,
    _size: usize,
) -> SysResult {
    let mut task_map = TASK_MAP.lock();
    let task = task_map.get_mut(&badge).unwrap();

    if clone_args.is_null() {
        return Err(Errno::EINVAL);
    }
    let clone_args: CloneArgs = read_item(task, clone_args)?;
    let clone_flags = CloneFlags::from_bits(clone_args.flags).ok_or(Errno::EINVAL)?;

    // Default to clone without any flags
    let mut new_task = Sel4Task::new().map_err(|_| Errno::ENOMEM)?;
    if !clone_flags.contains(CloneFlags::CLONE_VM) {
        // Copy vspace to child
        clone_vspace(&mut new_task, &task);
    } else {
        let new_slot = alloc_slot();
        let new_vspace = new_slot.cap::<cap_type::VSpace>();

        new_slot
            .copy_from(&task.vspace.into(), CapRights::all())
            .unwrap();

        new_task.vspace = new_vspace;
    }

    // 配置新任务 IPC Buffer
    let ipc_buffer_cap = PhysPage::new(alloc_page());
    let ipc_buffer_addr = 0x4_0000;
    new_task.map_page(ipc_buffer_addr as _, ipc_buffer_cap);
    // 配置新任务 IPC 缓冲区 和 上下文
    new_task
        .tcb
        .tcb_configure(
            fault_ep.cptr(),
            new_task.cnode,
            CNodeCapData::new(0, sel4::WORD_SIZE - CNODE_RADIX_BITS),
            new_task.vspace,
            ipc_buffer_addr,
            ipc_buffer_cap.cap(),
        )
        .map_err(|_| Errno::ENOMEM)?;
    new_task
        .tcb
        .tcb_set_sched_params(init_thread::slot::TCB.cap(), 0, 255)
        .map_err(|_| Errno::ENOMEM)?;

    let mut regs = task.tcb.tcb_read_all_registers(false).unwrap();
    if clone_args.init_fn.is_null() {
        *regs.pc_mut() += 4;
    } else {
        *regs.pc_mut() = clone_args.init_fn as _;
        *regs.gpr_mut(0) = clone_args.init_argv as _;
    }
    if !clone_args.stack.is_null() {
        *regs.sp_mut() = clone_args.stack as _;
    }

    new_task
        .tcb
        .tcb_write_all_registers(false, &mut regs)
        .unwrap();
    new_task.tcb.tcb_resume().unwrap();
    task_map.insert(new_task.id as _, new_task);

    Ok(badge as usize)
}

fn clone_vspace(dst: &mut Sel4Task, src: &Sel4Task) {
    /// free page placeholder
    static mut EXT_FREE_PAGE_PLACEHOLDER: FreePagePlaceHolder =
        FreePagePlaceHolder([0; GRANULE_SIZE]);

    for (vaddr, page_cap) in src.mapped_page.iter() {
        let new_page_cap = PhysPage::new(alloc_page());

        // READ data from src page to new_page
        new_page_cap
            .cap()
            .frame_map(
                init_thread::slot::VSPACE.cap(),
                core::ptr::addr_of!(EXT_FREE_PAGE_PLACEHOLDER) as _,
                CapRights::all(),
                VmAttributes::DEFAULT,
            )
            .unwrap();

        let temp_cap = Cap::<sel4::cap_type::SmallPage>::from_bits(0);
        init_thread::slot::CNODE
            .cap()
            .absolute_cptr(temp_cap)
            .copy(
                &init_thread::slot::CNODE.cap().absolute_cptr(page_cap.cap()),
                CapRights::all(),
            )
            .unwrap();

        temp_cap
            .frame_map(
                init_thread::slot::VSPACE.cap(),
                page_seat_vaddr(),
                CapRights::all(),
                VmAttributes::DEFAULT,
            )
            .unwrap();

        unsafe {
            core::ptr::copy(
                page_seat_vaddr() as *const u8,
                core::ptr::addr_of!(EXT_FREE_PAGE_PLACEHOLDER) as *mut u8,
                GRANULE_SIZE,
            );
        }

        temp_cap.frame_unmap().unwrap();

        init_thread::slot::CNODE
            .cap()
            .absolute_cptr(temp_cap)
            .delete()
            .unwrap();

        new_page_cap.cap().frame_unmap().unwrap();

        dst.map_page(*vaddr, new_page_cap);
    }
}
