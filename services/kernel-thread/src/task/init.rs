use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use common::config::{CNODE_RADIX_BITS, DEFAULT_PARENT_EP, PAGE_SIZE, STACK_ALIGN_SIZE};
use libc_core::elf::AuxType;
use memory_addr::MemoryAddr;
use sel4::{CNodeCapData, init_thread::slot};
use sel4_kit::slot_manager::LeafSlot;

use crate::consts::task::DEF_STACK_TOP;

use super::Sel4Task;

impl Sel4Task {
    /// 初始化用户栈，传递参数(args)，环境变量(env)和辅助向量(auxv)
    ///
    /// 需要传递的栈，环境变量和辅助向量需要提前填充到 [Sel4Task::info] 下
    pub fn init_stack(&self) -> usize {
        // Other Strings 以 STACK_ALIGN_SIZE 对齐
        //
        // +------------------+  <- 用户栈顶
        // │    EnvArg Strings│
        // +------------------+
        // │    0             │
        // +------------------+
        // │    AuxNull(0)    │
        // +------------------+
        // │    AuxValue      │
        // +------------------+
        // │    AuxKey        │
        // +------------------+
        // │    ...           |
        // +------------------+
        // │    0             │
        // +------------------+
        // │    EnvPtr...     │
        // +------------------+
        // │    0             │
        // +------------------+
        // │    ArgPtr...     │
        // +------------------+
        // │    ArgLen        │
        // +------------------+
        let mut stack_ptr = DEF_STACK_TOP;

        let mem = self.mem.lock();
        let mut page_writer = mem
            .mapped_page
            .get(&(DEF_STACK_TOP - PAGE_SIZE))
            .unwrap()
            .lock();

        let args_ptr: Vec<_> = self
            .info
            .lock()
            .args
            .iter()
            .map(|arg| {
                // TODO: set end bit was zeroed manually.
                stack_ptr = (stack_ptr - arg.len() - 1).align_down(STACK_ALIGN_SIZE);
                page_writer.write_bytes(stack_ptr, arg.as_bytes());
                page_writer.write_u8(stack_ptr + arg.len(), 0);
                stack_ptr
            })
            .collect();

        let envs = [
            "LD_LIBRARY_PATH=/",
            "PS1=\x1b[1m\x1b[32mrelk\x1b[0m:\x1b[1m\x1b[34m\\w\x1b[0m\\$ \0",
            "PATH=/:/bin:/usr/bin",
            "UB_BINDIR=./",
        ];
        let envps: Vec<_> = envs
            .iter()
            .map(|env| {
                stack_ptr = (stack_ptr - env.len() - 1).align_down(STACK_ALIGN_SIZE);
                page_writer.write_bytes(stack_ptr, env.as_bytes());
                page_writer.write_u8(stack_ptr + env.len(), 0);
                stack_ptr
            })
            .collect();

        let mut push_num = |num: usize| {
            stack_ptr -= core::mem::size_of::<usize>();
            page_writer.write_usize(stack_ptr, num);
        };

        let mut auxv = BTreeMap::new();
        auxv.insert(AuxType::ExecFn, args_ptr[0]);
        auxv.insert(AuxType::PageSize, PAGE_SIZE);
        auxv.insert(AuxType::Entry, self.info.lock().entry);
        auxv.insert(AuxType::GID, 0);
        auxv.insert(AuxType::EGID, 0);
        auxv.insert(AuxType::UID, 0);
        auxv.insert(AuxType::EUID, 0);
        auxv.insert(AuxType::Null, 0);

        // push auxiliary vector
        for (key, v) in auxv.into_iter() {
            push_num(v);
            push_num(key as usize);
        }
        // push environment
        push_num(0);
        envps.iter().rev().for_each(|x| push_num(*x));
        // push args pointer
        push_num(0);
        args_ptr.iter().rev().for_each(|x| push_num(*x));
        // push argv
        push_num(args_ptr.len());
        stack_ptr
    }

    /// 初始化 [sel4::cap::Tcb] 信息
    ///
    /// 初始化 tcb 信息，设置调度优先级，目前设定为固定值
    /// TODO: 更改调度优先级的设置，更加灵活自由
    pub fn init_tcb(&self) -> Result<(), sel4::Error> {
        self.tcb.tcb_configure(
            DEFAULT_PARENT_EP.cptr(),
            self.cnode,
            CNodeCapData::new(0, sel4::WORD_SIZE - CNODE_RADIX_BITS),
            self.vspace,
            0,
            LeafSlot::new(0).cap(),
        )?;
        self.tcb.tcb_set_sched_params(slot::TCB.cap(), 0, 255)
    }
}
