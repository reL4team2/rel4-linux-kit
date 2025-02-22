use crate::{consts::task::DEF_STACK_TOP, task::Sel4Task, utils::obj::alloc_page};
use alloc::{collections::btree_map::BTreeMap, string::String};
use common::page::PhysPage;
use core::cmp;
use crate_consts::{CNODE_RADIX_BITS, DEFAULT_PARENT_EP, PAGE_SIZE};
use include_bytes_aligned::include_bytes_aligned;
use object::{BinaryFormat, File, Object, ObjectSection};
use sel4::{init_thread::slot, CNodeCapData, CPtr, Result};
use spin::Mutex;

// TODO: Make elf file path dynamically available.
#[cfg(not(feature = "example"))]
const CHILD_ELF: &[u8] = include_bytes_aligned!(16, "../../../.env/busybox-ins.elf");
#[cfg(feature = "example")]
const CHILD_ELF: &[u8] = include_bytes_aligned!(16, "../../../.env/example");

/// 任务表，可以通过任务 ID 获取任务
pub static TASK_MAP: Mutex<BTreeMap<u64, Sel4Task>> = Mutex::new(BTreeMap::new());

/// 添加一个测试任务
pub fn add_test_child() -> Result<()> {
    let args = &["busybox", "echo", "Kernel Thread's Child Says Hello!"];
    // let args = &["busybox"];
    // let args = &["busybox", "sh"];
    let mut task = Sel4Task::new()?;

    task.load_elf(CHILD_ELF);

    let file = File::parse(CHILD_ELF).expect("can't load elf file");
    assert!(file.format() == BinaryFormat::Elf);

    // 填充初始化信息
    task.info.entry = file.entry() as _;
    task.info.args = args.iter().map(|x| String::from(*x)).collect();

    // 映射栈内存并填充初始化信息
    task.map_region(DEF_STACK_TOP - 16 * PAGE_SIZE, DEF_STACK_TOP);
    let sp_ptr = task.init_stack();

    let ipc_buf_page = PhysPage::new(alloc_page());
    let ipc_buffer_addr = file
        .sections()
        .fold(0, |acc, x| cmp::max(acc, x.address() + x.size()))
        .div_ceil(PAGE_SIZE as _)
        * PAGE_SIZE as u64;
    task.map_page(ipc_buffer_addr as _, ipc_buf_page);

    // 配置程序最大的位置
    task.info.task_vm_end = ipc_buffer_addr as usize + PAGE_SIZE;

    // 配置子任务
    task.tcb.tcb_configure(
        CPtr::from_bits(DEFAULT_PARENT_EP),
        task.cnode,
        CNodeCapData::new(0, sel4::WORD_SIZE - CNODE_RADIX_BITS),
        task.vspace,
        ipc_buffer_addr,
        ipc_buf_page.cap(),
    )?;
    task.tcb.tcb_set_sched_params(slot::TCB.cap(), 0, 255)?;

    // 写入线程的寄存器信息
    {
        let mut user_context = sel4::UserContext::default();

        *user_context.pc_mut() = task.info.entry as _;
        *user_context.sp_mut() = sp_ptr as _;

        // 写入寄存器信息并恢复运行
        task.tcb
            .tcb_write_all_registers(true, &mut user_context)
            .unwrap();
    }

    TASK_MAP.lock().insert(task.id as _, task);

    Ok(())
}
