use crate::{
    consts::task::DEF_STACK_TOP,
    exception::aux_thread,
    fs::{file::File, stdio::StdConsole},
    task::Sel4Task,
    utils,
};
use alloc::{boxed::Box, collections::btree_map::BTreeMap, string::String, sync::Arc};
use common::thread::create_thread;
use crate_consts::PAGE_SIZE;
use object::{BinaryFormat, Object};
use sel4::{IpcBuffer, Result, UserContext, set_ipc_buffer};
use sel4_initialize_tls::ContArg;
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
    let _ = file_table.add_at(
        0,
        Arc::new(Mutex::new(File::from_raw(Box::new(StdConsole::new(0))))),
    );
    let _ = file_table.add_at(
        1,
        Arc::new(Mutex::new(File::from_raw(Box::new(StdConsole::new(1))))),
    );
    let _ = file_table.add_at(
        2,
        Arc::new(Mutex::new(File::from_raw(Box::new(StdConsole::new(2))))),
    );
    drop(file_table);

    // 写入线程的寄存器信息
    {
        let mut user_context = sel4::UserContext::default();

        *user_context.pc_mut() = task.info.entry as _;
        *user_context.sp_mut() = sp_ptr as _;

        // 写入寄存器信息并恢复运行
        task.tcb
            .tcb_write_all_registers(false, &mut user_context)
            .unwrap();
    }

    TASK_MAP.lock().insert(task.id as _, task);

    Ok(())
}

/// 启动辅助任务
pub fn create_aux_thread() {
    let tcb = utils::obj::alloc_tcb();
    let ipc_cap = utils::obj::alloc_page();
    let sp_cap = utils::obj::alloc_page();
    utils::page::map_page_self(0x6_0000_0000, ipc_cap);
    utils::page::map_page_self(0x6_0000_1000, sp_cap);
    let mut ctx = UserContext::default();
    *ctx.pc_mut() = test_func as u64;
    *ctx.sp_mut() = 0x6_0000_1000;
    *ctx.c_param_mut(0) = 0x6_0000_0000;
    create_thread(tcb, ctx, 0x6_0000_0000, ipc_cap).unwrap();
}

/// 辅助任务入口函数
extern "C" fn test_func(cont_arg: *mut ContArg) {
    let const_fn = |ipc_addr: *mut ContArg| -> ! {
        set_ipc_buffer(unsafe { ipc_addr.cast::<IpcBuffer>().as_mut().unwrap() });
        aux_thread()
    };
    unsafe { sel4_runtime_common::initialize_tls_on_stack_and_continue(const_fn, cont_arg) };
}
