//! 任务相关接口
//!
//! 本接口中包含 Task 结构体的定义和实现    
mod auxv;
mod info;
mod init;

use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use common::page::PhysPage;
use core::{
    cmp,
    sync::atomic::{AtomicU64, Ordering},
};
use crate_consts::{CNODE_RADIX_BITS, DEFAULT_PARENT_EP, DEFAULT_SERVE_EP, PAGE_SIZE};
use info::TaskInfo;
use sel4::{
    init_thread::{self, slot},
    CapRights, Error, VmAttributes,
};
use slot_manager::LeafSlot;
use xmas_elf::{program, ElfFile};

use crate::{
    consts::task::DEF_HEAP_ADDR,
    utils::obj::{alloc_cnode, alloc_page, alloc_pt, alloc_tcb, alloc_vspace},
};

/// Sel4Task 结构体
pub struct Sel4Task {
    /// 进程 ID
    pub pid: usize,
    /// 任务 ID (线程 ID)
    pub id: usize,
    /// 进程控制块（Capability)
    pub tcb: sel4::cap::Tcb,
    /// 能力空间入口 (CSpace Root)
    pub cnode: sel4::cap::CNode,
    /// 地址空间 (Capability)
    pub vspace: sel4::cap::VSpace,
    /// 已经映射的页表
    pub mapped_pt: Vec<sel4::cap::PT>,
    /// 已经映射的页
    pub mapped_page: BTreeMap<usize, PhysPage>,
    /// 堆地址，方便进行堆增长
    pub heap: usize,
    /// 退出状态码
    pub exit: Option<i32>,
    /// The clear thread tid field
    ///
    /// See <https://manpages.debian.org/unstable/manpages-dev/set_tid_address.2.en.html#clear_child_tid>
    ///
    /// When the thread exits, the kernel clears the word at this address if it is not NULL.
    pub clear_child_tid: Option<usize>,
    /// 任务初始信息，任务的初始信息记录在这里，方便进行初始化
    pub info: TaskInfo,
}

impl Drop for Sel4Task {
    fn drop(&mut self) {
        let root_cnode = init_thread::slot::CNODE.cap();
        root_cnode.absolute_cptr(self.tcb).revoke().unwrap();
        root_cnode.absolute_cptr(self.tcb).delete().unwrap();
        root_cnode.absolute_cptr(self.cnode).revoke().unwrap();
        root_cnode.absolute_cptr(self.cnode).delete().unwrap();
        root_cnode.absolute_cptr(self.vspace).revoke().unwrap();
        root_cnode.absolute_cptr(self.vspace).delete().unwrap();

        self.mapped_pt.iter().for_each(|cap| {
            root_cnode.absolute_cptr(*cap).revoke().unwrap();
            root_cnode.absolute_cptr(*cap).delete().unwrap();
        });
        self.mapped_page.values().for_each(|phys_page| {
            root_cnode.absolute_cptr(phys_page.cap()).revoke().unwrap();
            root_cnode.absolute_cptr(phys_page.cap()).delete().unwrap();
        });
    }
}

impl Sel4Task {
    /// 创建一个新的任务
    pub fn new() -> Result<Self, sel4::Error> {
        static ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        let tid = ID_COUNTER.fetch_add(1, Ordering::SeqCst) as usize;
        let vspace = alloc_vspace();
        let tcb = alloc_tcb();
        let cnode = alloc_cnode(CNODE_RADIX_BITS);
        slot::ASID_POOL.cap().asid_pool_assign(vspace).unwrap();

        // 构建 CSpace 需要的结构
        cnode
            .absolute_cptr_from_bits_with_depth(1, CNODE_RADIX_BITS)
            .copy(&LeafSlot::from_cap(tcb).abs_cptr(), CapRights::all())
            .unwrap();

        // Copy EndPoint to child
        cnode
            .absolute_cptr_from_bits_with_depth(DEFAULT_PARENT_EP, CNODE_RADIX_BITS)
            .mint(
                &LeafSlot::from(DEFAULT_SERVE_EP).abs_cptr(),
                CapRights::all(),
                tid as u64,
            )?;

        Ok(Sel4Task {
            id: tid,
            pid: 0,
            tcb,
            cnode,
            vspace,
            mapped_pt: Vec::new(),
            mapped_page: BTreeMap::new(),
            heap: DEF_HEAP_ADDR,
            exit: None,
            clear_child_tid: None,
            info: TaskInfo::default(),
        })
    }

    /// 在当前任务的地址空间中找到最大可用的虚拟地址
    ///
    /// - `start` 从哪块内存开始
    /// - `size`  需要查找的内存块大小
    pub fn find_free_area(&self, start: usize, size: usize) -> usize {
        let mut last_addr = self.info.task_vm_end.max(start);
        for (vaddr, _page) in &self.mapped_page {
            if last_addr + size <= *vaddr {
                return last_addr;
            }
            last_addr = *vaddr + PAGE_SIZE;
        }
        last_addr
    }

    /// 映射一个物理页到虚拟地址空间
    ///
    /// - `vaddr` 需要映射的虚拟地址，需要对齐到 4k 页
    /// - `page`  需要映射的物理页，是一个 Capability
    pub fn map_page(&mut self, vaddr: usize, page: PhysPage) {
        assert_eq!(vaddr % PAGE_SIZE, 0);
        for _ in 0..sel4::vspace_levels::NUM_LEVELS {
            let res: core::result::Result<(), sel4::Error> = page.cap().frame_map(
                self.vspace,
                vaddr as _,
                CapRights::all(),
                VmAttributes::DEFAULT,
            );
            match res {
                Ok(_) => {
                    self.mapped_page.insert(vaddr, page);
                    return;
                }
                Err(Error::FailedLookup) => {
                    let pt_cap = alloc_pt();
                    pt_cap
                        .pt_map(self.vspace, vaddr, VmAttributes::DEFAULT)
                        .unwrap();
                    self.mapped_pt.push(pt_cap);
                }
                _ => res.unwrap(),
            }
        }
    }

    /// 取消映射一个地址的物理页
    ///
    /// - `vaddr` 需要取消映射的虚拟地址，需要对齐到 4k 页
    pub fn unmap_page(&mut self, vaddr: usize) {
        assert_eq!(vaddr % PAGE_SIZE, 0);
        if let Some(page) = self.mapped_page.remove(&vaddr) {
            page.cap().frame_unmap().unwrap();
        }
    }

    /// 映射一个区域的内存
    ///
    /// - `start` 是起始地址
    /// - `end`   是结束地址
    ///
    /// 说明: 地址需要对齐到 0x1000
    pub fn map_region(&mut self, start: usize, end: usize) {
        assert!(end % 0x1000 == 0);
        assert!(start % 0x1000 == 0);

        for vaddr in (start..end).step_by(PAGE_SIZE) {
            let page_cap = PhysPage::new(alloc_page());
            self.map_page(vaddr, page_cap);
        }
    }

    /// 加载一个 elf 文件到当前任务的地址空间
    ///
    /// - `elf_data` 是 elf 文件的数据
    pub fn load_elf(&mut self, elf_data: &[u8]) {
        let file = ElfFile::new(elf_data).expect("This is not a valid elf file");

        // 从 elf 文件中读取数据
        file.program_iter()
            .filter(|ph| ph.get_type() == Ok(program::Type::Load))
            .for_each(|ph| {
                let mut offset = ph.offset() as usize;
                let mut vaddr = ph.virtual_addr() as usize;
                let end = offset + ph.file_size() as usize;
                let vaddr_end = vaddr + ph.mem_size() as usize;

                while vaddr < vaddr_end {
                    let voffset = vaddr % PAGE_SIZE;
                    let page_cap = match self.mapped_page.remove(&(vaddr / PAGE_SIZE * PAGE_SIZE)) {
                        Some(page_cap) => {
                            page_cap.cap().frame_unmap().unwrap();
                            page_cap
                        }
                        None => PhysPage::new(alloc_page()),
                    };

                    // 将 elf 中特定段的内容写入对应的物理页中
                    if offset < end {
                        let rsize = cmp::min(PAGE_SIZE - vaddr % PAGE_SIZE, end - offset);
                        page_cap.lock()[voffset..voffset + rsize]
                            .copy_from_slice(&elf_data[offset..offset + rsize]);
                        offset += rsize;
                    }

                    self.map_page(vaddr / PAGE_SIZE * PAGE_SIZE, page_cap);
                    self.mapped_page
                        .insert(vaddr / PAGE_SIZE * PAGE_SIZE, page_cap);

                    // Calculate offset
                    vaddr += PAGE_SIZE - vaddr % PAGE_SIZE;
                }
            });
    }

    /// 进行 brk 操作
    ///
    /// - `value` 是需要调整的堆地址
    ///
    /// ### 说明
    /// 如果 `value` 的值为 0，则返回当前的堆地址，否则就将堆扩展到指定的地址
    pub fn brk(&mut self, value: usize) -> usize {
        if value == 0 {
            return self.heap;
        }
        for vaddr in (self.heap..value).step_by(PAGE_SIZE) {
            let page_cap = PhysPage::new(alloc_page());
            self.map_page(vaddr, page_cap);
        }
        self.heap = value;
        value
    }

    /// 在当前任务 [Sel4Task] 的地址空间 [Sel4Task::vspace] 下读取特定地址的指令
    ///
    /// - `vaddr` 是需要读取指令的虚拟地址
    ///
    /// 说明：
    /// - 如果地址空间不存在或者地址未映射，返回 [Option::None]
    pub fn read_ins(&self, vaddr: usize) -> Option<u32> {
        self.mapped_page
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
    ///
    /// 说明：
    /// - 如果地址空间不存在或者地址未映射，返回 [Option::None]
    /// TODO: 跨物理页处理
    pub fn read_bytes(&self, vaddr: usize, len: usize) -> Option<Vec<u8>> {
        self.mapped_page
            .get(&(vaddr / PAGE_SIZE * PAGE_SIZE))
            .map(|page| {
                let offset = vaddr % PAGE_SIZE;
                let len = cmp::min(len, PAGE_SIZE - offset);
                page.lock()[offset..offset + len].to_vec()
            })
    }
}
