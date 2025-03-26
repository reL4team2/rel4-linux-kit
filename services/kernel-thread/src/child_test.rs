use crate::{
    consts::task::DEF_STACK_TOP,
    fs::{file::File, stdio::StdConsole},
    task::Sel4Task,
};
use alloc::{boxed::Box, collections::btree_map::BTreeMap, string::String, sync::Arc};
use config::PAGE_SIZE;
use object::{BinaryFormat, Object};
use sel4::Result;
use spin::Mutex;

/// 任务表，可以通过任务 ID 获取任务
pub static TASK_MAP: Mutex<BTreeMap<u64, Sel4Task>> = Mutex::new(BTreeMap::new());

/// 添加一个测试任务
pub fn add_test_child(elf_file: &[u8], args: &[&str]) -> Result<()> {
    let mut task = Sel4Task::new()?;

    let file: object::File<'_> = object::File::parse(elf_file).expect("can't load elf file");
    assert!(file.format() == BinaryFormat::Elf);

    task.load_elf(&file);

    // 填充初始化信息
    task.info.entry = file.entry() as _;
    task.info.args = args.iter().map(|x| String::from(*x)).collect();

    // 映射栈内存并填充初始化信息
    task.map_region(DEF_STACK_TOP - 16 * PAGE_SIZE, DEF_STACK_TOP);
    let sp_ptr = task.init_stack();

    // 配置子任务
    task.init_tcb()?;

    let mut file_table = task.file.file_ds.lock();
    for i in 0..3 {
        let file = File::from_raw(Box::new(StdConsole::new(i)));
        let _ = file_table.add_at(i as _, Arc::new(Mutex::new(file)));
    }
    drop(file_table);

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
