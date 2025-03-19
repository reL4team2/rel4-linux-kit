//! 进程内存相关的模块
//!
//!

use core::cmp;

use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use common::page::PhysPage;
use config::PAGE_SIZE;

use crate::{consts::task::DEF_HEAP_ADDR, utils::obj::alloc_page};

use super::Sel4Task;

pub struct TaskMemInfo {
    /// 已经映射的页表
    pub mapped_pt: Vec<sel4::cap::PT>,
    /// 已经映射的页
    pub mapped_page: BTreeMap<usize, PhysPage>,
    /// 堆地址，方便进行堆增长
    pub heap: usize,
}

impl Default for TaskMemInfo {
    fn default() -> Self {
        Self {
            mapped_pt: Default::default(),
            mapped_page: Default::default(),
            heap: DEF_HEAP_ADDR,
        }
    }
}

impl Sel4Task {
    /// 进行 brk 操作
    ///
    /// - `value` 是需要调整的堆地址
    ///
    /// ### 说明
    /// 如果 `value` 的值为 0，则返回当前的堆地址，否则就将堆扩展到指定的地址
    pub fn brk(&mut self, value: usize) -> usize {
        let mut mem_info = self.mem.lock();
        if value == 0 {
            return mem_info.heap;
        }
        let origin = mem_info.heap;
        mem_info.heap = value;
        drop(mem_info);
        for vaddr in (origin..value).step_by(PAGE_SIZE) {
            let page_cap = PhysPage::new(alloc_page());
            self.map_page(vaddr / PAGE_SIZE * PAGE_SIZE, page_cap);
        }
        value
    }

    /// 在当前任务 [Sel4Task] 的地址空间 [Sel4Task::vspace] 下读取特定地址的指令
    ///
    /// - `vaddr` 是需要读取指令的虚拟地址
    ///
    /// 说明：
    /// - 如果地址空间不存在或者地址未映射，返回 [Option::None]
    pub fn read_ins(&self, vaddr: usize) -> Option<u32> {
        self.mem
            .lock()
            .mapped_page
            .get(&(vaddr / PAGE_SIZE * PAGE_SIZE))
            .map(|page| {
                let offset = vaddr % PAGE_SIZE;
                let ins = page.lock()[offset..offset + 4].try_into().unwrap();
                u32::from_le_bytes(ins)
            })
    }

    /// 在当前任务 [Sel4Task] 的地址空间 [Sel4Task::vspace] 下读取特定地址的数据
    ///
    /// - `vaddr` 是需要读取数据的虚拟地址
    /// - `len`   是需要读取的数据长度
    ///
    /// 说明：
    /// - 如果地址空间不存在或者地址未映射，返回 [Option::None]
    pub fn read_bytes(&self, mut vaddr: usize, len: usize) -> Option<Vec<u8>> {
        let mut data = Vec::new();
        let mem_info = self.mem.lock();
        let vaddr_end = vaddr + len;
        while vaddr < vaddr_end {
            let page = mem_info.mapped_page.get(&(vaddr / PAGE_SIZE * PAGE_SIZE))?;
            let offset = vaddr % PAGE_SIZE;
            let rsize = cmp::min(PAGE_SIZE - offset, vaddr_end - vaddr);
            data.extend_from_slice(&page.lock()[offset..offset + rsize]);
            vaddr += rsize;
        }
        Some(data)
    }

    /// 在当前任务 [Sel4Task] 的地址空间 [Sel4Task::vspace] 下读取 C 语言的字符串信息，直到遇到 \0
    ///
    /// - `vaddr` 是需要读取数据的虚拟地址
    ///
    /// 说明：
    /// - 如果地址空间不存在或者地址未映射，返回 [Option::None]
    pub fn read_cstr(&self, mut vaddr: usize) -> Option<Vec<u8>> {
        let mut data = Vec::new();
        let mem_info = self.mem.lock();
        loop {
            let page = mem_info.mapped_page.get(&(vaddr / PAGE_SIZE * PAGE_SIZE))?;
            let offset = vaddr % PAGE_SIZE;
            let position = page.lock()[offset..].iter().position(|x| *x == 0);

            if let Some(position) = position {
                data.extend_from_slice(&page.lock()[offset..offset + position]);
                break;
            }
            data.extend_from_slice(&page.lock()[offset..]);
            vaddr += PAGE_SIZE - offset;
        }
        Some(data)
    }

    /// 在当前任务 [Sel4Task] 的地址空间 [Sel4Task::vspace] 下读取 C 语言的字符串信息，直到遇到 \0
    ///
    /// - `vaddr` 是需要读取数据的虚拟地址
    ///
    /// 说明：
    /// - 如果地址空间不存在或者地址未映射，返回 [Option::None]
    pub fn read_vec(&self, mut vaddr: usize) -> Option<Vec<usize>> {
        let mut data = Vec::new();
        let mem_info = self.mem.lock();
        loop {
            let page = mem_info.mapped_page.get(&(vaddr / PAGE_SIZE * PAGE_SIZE))?;
            let mut offset = vaddr % PAGE_SIZE;
            while offset < PAGE_SIZE {
                let value = page.lock().read_usize(offset);
                if value == 0 {
                    return Some(data);
                }
                offset += size_of::<usize>();
                data.push(value);
            }
            vaddr += PAGE_SIZE - offset;
        }
    }

    /// 在当前任务 [Sel4Task] 的地址空间 [Sel4Task::vspace] 下写入数据到特定地址
    ///
    /// - `vaddr` 是需要写入数据的虚拟地址
    /// - `data`  是需要写入的数据
    ///
    /// 说明：
    /// - 如果地址空间不存在或者地址未映射，返回 [Option::None]
    ///   TODO: 在写入之前检测所有的地址是否可以写入
    pub fn write_bytes(&self, mut vaddr: usize, data: &[u8]) -> Option<()> {
        let mem_info = self.mem.lock();
        let vaddr_end = vaddr + data.len();
        while vaddr < vaddr_end {
            let page = mem_info.mapped_page.get(&(vaddr / PAGE_SIZE * PAGE_SIZE))?;
            let offset = vaddr % PAGE_SIZE;
            let rsize = cmp::min(PAGE_SIZE - offset, vaddr_end - vaddr);
            page.lock()[offset..offset + rsize].copy_from_slice(&data[..rsize]);
            vaddr += rsize;
        }
        Some(())
    }

    /// 清理映射的内存
    ///
    /// 如果有已经映射的内存, 清理
    pub fn clear_maped(&self) {
        self.mem
            .lock()
            .mapped_page
            .values()
            .for_each(|x| x.cap().frame_unmap().unwrap());
        self.mem.lock().mapped_page.clear();
    }
}
