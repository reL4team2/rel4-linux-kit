use crate::{consts::task::DEF_STACK_TOP, fs::file::File, task::Sel4Task};
use alloc::{collections::btree_map::BTreeMap, string::String};
use core::cmp;
use crate_consts::{CNODE_RADIX_BITS, DEFAULT_PARENT_EP, PAGE_SIZE};
use object::{BinaryFormat, Object, ObjectSection};
use sel4::{init_thread::slot, CNodeCapData, Result};
use slot_manager::LeafSlot;
use spin::Mutex;

// TODO: Make elf file path dynamically available.
// const CHILD_ELF: &[u8] = include_bytes_aligned!(16, "../../../.env/testcases/chdir");

/// 任务表，可以通过任务 ID 获取任务
pub static TASK_MAP: Mutex<BTreeMap<u64, Sel4Task>> = Mutex::new(BTreeMap::new());

/// 添加一个测试任务
pub fn add_test_child() -> Result<()> {
    let elf_file = File::open("/chdir", 0).unwrap().read_all().unwrap();
    // let args = &["busybox", "echo", "Kernel Thread's Child Says Hello!"];
    // let args = &["busybox"];
    let args = &["busybox", "sh"];
    // let args = &["busybox", "printenv"];
    // let args = &["busybox", "uname", "-a"];
    let mut task = Sel4Task::new()?;

    task.load_elf(&elf_file);

    let file = object::File::parse(elf_file.as_slice()).expect("can't load elf file");
    assert!(file.format() == BinaryFormat::Elf);

    // 填充初始化信息
    task.info.entry = file.entry() as _;
    task.info.args = args.iter().map(|x| String::from(*x)).collect();

    // 映射栈内存并填充初始化信息
    task.map_region(DEF_STACK_TOP - 16 * PAGE_SIZE, DEF_STACK_TOP);
    let sp_ptr = task.init_stack();

    // 配置程序最大的位置
    task.info.task_vm_end = file
        .sections()
        .fold(0, |acc, x| cmp::max(acc, x.address() + x.size()))
        .div_ceil(PAGE_SIZE as _) as usize
        * PAGE_SIZE;

    // 配置子任务
    task.tcb.tcb_configure(
        DEFAULT_PARENT_EP.cptr(),
        task.cnode,
        CNodeCapData::new(0, sel4::WORD_SIZE - CNODE_RADIX_BITS),
        task.vspace,
        0,
        LeafSlot::new(0).cap(),
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
