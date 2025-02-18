use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use common::USPACE_STACK_TOP;
use crate_consts::{PAGE_SIZE, STACK_ALIGN_SIZE};
use memory_addr::MemoryAddr;

use super::{auxv::AuxV, Sel4Task};

impl Sel4Task {
    /// 初始化用户栈，传递参数(args)，环境变量(env)和辅助向量(auxv)
    fn init_stack(&mut self) {
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
        let mut stack_ptr = USPACE_STACK_TOP;
        let mut page_writer = self.mapped_page.get(&USPACE_STACK_TOP).unwrap().lock();

        let args_ptr: Vec<_> = self
            .info
            .args
            .iter()
            .map(|arg| {
                // TODO: set end bit was zeroed manually.
                stack_ptr = (stack_ptr - arg.bytes().len()).align_down(STACK_ALIGN_SIZE);
                page_writer.write_bytes(stack_ptr, arg.as_bytes());
                page_writer.write_u8(stack_ptr + arg.as_bytes().len(), 0);
                stack_ptr
            })
            .collect();

        let mut push_num = |num: usize| {
            stack_ptr -= core::mem::size_of::<usize>();
            page_writer.write_usize(stack_ptr, num);
        };

        let mut auxv = BTreeMap::new();
        auxv.insert(AuxV::EXECFN, args_ptr[0]);
        auxv.insert(AuxV::PAGESZ, PAGE_SIZE);
        auxv.insert(AuxV::ENTRY, self.info.entry);
        auxv.insert(AuxV::GID, 0);
        auxv.insert(AuxV::EGID, 0);
        auxv.insert(AuxV::UID, 0);
        auxv.insert(AuxV::EUID, 0);
        auxv.insert(AuxV::NULL, 0);

        // push auxiliary vector
        for (key, v) in auxv.into_iter() {
            push_num(v);
            push_num(key as usize);
        }
        // push environment
        push_num(0);
        // push args pointer
        push_num(0);
        args_ptr.iter().rev().for_each(|x| push_num(*x));
        // push argv
        push_num(args_ptr.len());
    }
}
