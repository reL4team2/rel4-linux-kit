use crate::{
    syscall::handle_ipc_call,
    task::Sel4Task,
    utils::{align_bits, obj::alloc_page},
};
use alloc::collections::btree_map::BTreeMap;
use common::{page::PhysPage, CustomMessageLabel, USPACE_STACK_TOP};
use core::cmp;
use crate_consts::{CNODE_RADIX_BITS, DEFAULT_SERVE_EP, PAGE_SIZE, PAGE_SIZE_BITS};
use object::{BinaryFormat, File};
use sel4::{
    debug_println, init_thread::slot, reply, with_ipc_buffer, with_ipc_buffer_mut, CNodeCapData,
    Fault, MessageInfo, Result, Word,
};
use slot_manager::LeafSlot;
use spin::Mutex;
use xmas_elf::ElfFile;

// TODO: Make elf file path dynamically available.
const CHILD_ELF: &[u8] = include_bytes!("../../../target/test-thread.elf");

pub static TASK_MAP: Mutex<BTreeMap<u64, Sel4Task>> = Mutex::new(BTreeMap::new());

pub fn test_child() -> Result<()> {
    let ep = LeafSlot::new(DEFAULT_SERVE_EP as _).cap();
    let args = &["busybox", "echo", "Kernel Thread's Child Says Hello!"];
    debug_println!("[KernelThread] Child Task Start, busybox args: {:?}", args);
    let mut task = Sel4Task::new()?;

    debug_println!("[KernelThread] Child Task Mapping ELF...");
    task.load_elf(CHILD_ELF);

    let file = File::parse(CHILD_ELF).expect("can't load elf file");
    assert!(file.format() == BinaryFormat::Elf);
    loop {}
    let child_elf_file = ElfFile::new(CHILD_ELF).expect("[KernelThread] can't load elf file");

    let sp_ptr = task.map_stack(0, USPACE_STACK_TOP - 16 * PAGE_SIZE, USPACE_STACK_TOP, args);

    let ipc_buf_page = PhysPage::new(alloc_page());
    let max = child_elf_file
        .section_iter()
        .fold(0, |acc, x| cmp::max(acc, x.address() + x.size()));
    let ipc_buffer_addr = max.div_ceil(4096) * 4096;
    task.map_page(ipc_buffer_addr as _, ipc_buf_page);

    // Configure the child task
    task.tcb.tcb_configure(
        ep.cptr(),
        task.cnode,
        CNodeCapData::new(0, sel4::WORD_SIZE - CNODE_RADIX_BITS),
        task.vspace,
        ipc_buffer_addr,
        ipc_buf_page.cap(),
    )?;
    task.tcb.tcb_set_sched_params(slot::TCB.cap(), 0, 255)?;

    let mut user_context = sel4::UserContext::default();

    // Set child task's context
    *user_context.pc_mut() = child_elf_file.header.pt2.entry_point();
    *user_context.sp_mut() = sp_ptr as _;
    *user_context.gpr_mut(0) = ep.cptr().bits();
    // Get TSS section address.
    user_context.inner_mut().tpidr_el0 = child_elf_file
        .find_section_by_name(".tbss")
        .map_or(0, |tls| tls.address());

    task.tcb
        .tcb_write_all_registers(false, &mut user_context)
        .unwrap();

    task.tcb.tcb_resume().unwrap();

    TASK_MAP.lock().insert(task.id as _, task);

    loop {
        let (message, badge) = ep.recv(());

        if message.label() < 8 {
            let fault = with_ipc_buffer(|buffer| Fault::new(&buffer, &message));
            debug_println!("[Kernel Thread] Received Fault: {:#x?}", fault);
            match fault {
                Fault::VmFault(vmfault) => {
                    let vaddr = align_bits(vmfault.addr() as usize, PAGE_SIZE_BITS);
                    let page_cap = PhysPage::new(alloc_page());
                    let mut task_map = TASK_MAP.lock();
                    let task = task_map.get_mut(&badge).unwrap();
                    task.map_page(vaddr, page_cap);

                    task.tcb.tcb_resume().unwrap();
                    drop(task_map);
                }
                _ => {}
            }
        } else {
            match CustomMessageLabel::try_from(&message) {
                Some(CustomMessageLabel::TestCustomMessage) => reply_with(&[]),
                Some(CustomMessageLabel::SysCall) => {
                    let (sys_id, args) = with_ipc_buffer(|ipc_buf| {
                        let msgs = ipc_buf.msg_regs();
                        let args: [Word; 6] = msgs[1..7].try_into().unwrap();
                        (msgs[0] as _, args.map(|x| x as usize))
                    });
                    let res = handle_ipc_call(badge, sys_id, args, ep)
                        .map_err(|e| -e.into_raw() as isize)
                        .unwrap_or_else(|e| e as usize);
                    reply_with(&[res]);
                }
                Some(CustomMessageLabel::Exit) => break,
                None => {
                    debug_println!(
                        "[Kernel Thread] Recv unknown {} length message {:#x?} ",
                        message.length(),
                        message
                    );
                }
            }
        }
        sel4::r#yield();
    }

    // TODO: Free memory from slots.
    Ok(())
}

/// Reply a message with empty message information
#[inline]
pub(crate) fn reply_with(regs: &[usize]) {
    with_ipc_buffer_mut(|buffer| {
        let msg_regs = buffer.msg_regs_mut();
        regs.iter()
            .enumerate()
            .for_each(|(i, reg)| msg_regs[i] = *reg as _);
        reply(buffer, MessageInfo::new(0, 0, 0, 8 * regs.len()))
    });
}
