use core::task::{Poll, Waker};

use crate::{consts::task::DEF_STACK_TOP, fs::stdio::StdConsole, task::Sel4Task};
use alloc::{collections::btree_map::BTreeMap, string::String, sync::Arc, vec::Vec};
use common::config::PAGE_SIZE;
use fs::file::File;
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
        let file = File::new_dev(Arc::new(StdConsole::new(i)));
        let _ = file_table.add_at(i as _, file);
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

/// 等待队列 (父进程 id, 子进程 id)
pub static WAITING_PID: Mutex<Vec<(u64, u64, Waker)>> = Mutex::new(Vec::new());

/// 等待程序结束
///
/// (父进程 pid, 等待的子进程 pid, Blocking)
pub struct WaitPid(pub u64, pub u64, pub bool);

impl Future for WaitPid {
    type Output = Option<(u64, i32)>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let task_map = TASK_MAP.lock();
        let finded = task_map
            .iter()
            .find(|(_, target)| {
                target.exit.is_some() && target.ppid == self.0 as _ && target.pid == self.1 as _
            })
            .map(|(&tid, task)| (tid, task.exit.unwrap()));

        match finded {
            Some(res) => Poll::Ready(Some(res)),
            None => {
                if self.2 {
                    return Poll::Ready(None);
                }
                WAITING_PID
                    .lock()
                    .push((self.0, self.1, cx.waker().clone()));
                Poll::Pending
            }
        }
    }
}

/// 等待程序结束
///
/// (父进程 pid, poll once)
pub struct WaitAnyChild(pub u64, pub bool);

impl Future for WaitAnyChild {
    type Output = Option<(u64, i32)>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let task_map = TASK_MAP.lock();
        let finded = task_map
            .iter()
            .find(|(_, target)| target.exit.is_some() && target.ppid == self.0 as _)
            .map(|(&tid, task)| (tid, task.exit.unwrap()));

        match finded {
            Some(res) => Poll::Ready(Some(res)),
            None => {
                if self.1 {
                    return Poll::Ready(None);
                }
                WAITING_PID
                    .lock()
                    .push((self.0, -1 as _, cx.waker().clone()));
                Poll::Pending
            }
        }
    }
}

pub fn wake_hangs(task: &Sel4Task) {
    let mut queue = WAITING_PID.lock();
    let finded = queue
        .iter()
        .position(|x| x.0 == task.ppid as _ && (x.1 == u64::MAX || x.1 == task.pid as _));
    if let Some(idx) = finded {
        queue.remove(idx).2.wake();
    }
}
