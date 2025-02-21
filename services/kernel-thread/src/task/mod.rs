mod auxv;
mod info;
mod init;

use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use common::{page::PhysPage, USPACE_BASE};
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

use crate::utils::obj::{alloc_cnode, alloc_page, alloc_pt, alloc_tcb, alloc_vspace};

pub struct Sel4Task {
    pub pid: usize,
    pub id: usize,
    pub tcb: sel4::cap::Tcb,
    pub cnode: sel4::cap::CNode,
    pub vspace: sel4::cap::VSpace,
    pub mapped_pt: Vec<sel4::cap::PT>,
    pub mapped_page: BTreeMap<usize, PhysPage>,
    pub heap: usize,
    pub exit: Option<i32>,
    /// The clear thread tid field
    ///
    /// See <https://manpages.debian.org/unstable/manpages-dev/set_tid_address.2.en.html#clear_child_tid>
    ///
    /// When the thread exits, the kernel clears the word at this address if it is not NULL.
    pub clear_child_tid: Option<usize>,
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
            heap: 0x2_0000_0000,
            exit: None,
            clear_child_tid: None,
            info: TaskInfo::default(),
        })
    }

    /// To find a free area in the vspace.
    ///
    /// The area starts from `start` and the size is `size`.
    pub fn find_free_area(&self, start: usize, size: usize) -> Option<usize> {
        let mut last_addr = USPACE_BASE.max(start);
        for (vaddr, _page) in &self.mapped_page {
            if last_addr + size <= *vaddr {
                return Some(last_addr);
            }
            last_addr = *vaddr + PAGE_SIZE;
        }
        // TODO: Set the limit of the top of the user space.
        Some(last_addr)
    }

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

    pub fn unmap_page(&mut self, vaddr: usize, page: PhysPage) {
        assert_eq!(vaddr % PAGE_SIZE, 0);
        let res = page.cap().frame_unmap();
        match res {
            Ok(_) => {
                self.mapped_page.remove(&vaddr);
            }
            _ => res.unwrap(),
        }
    }

    pub fn map_region(&mut self, start: usize, end: usize) {
        assert!(end % 0x1000 == 0);
        assert!(start % 0x1000 == 0);

        for vaddr in (start..end).step_by(PAGE_SIZE) {
            let page_cap = PhysPage::new(alloc_page());
            self.map_page(vaddr, page_cap);
        }
    }

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
                        page_cap.lock()[..rsize].copy_from_slice(&elf_data[offset..offset + rsize]);
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

    pub fn brk(&mut self, value: usize) -> usize {
        if value == 0 {
            return self.heap;
        }
        for vaddr in (self.heap..value).step_by(PAGE_SIZE) {
            let page_cap = PhysPage::new(alloc_page());
            self.map_page(vaddr, page_cap);
        }
        value
    }

    pub fn read_ins(&self, vaddr: usize) -> Option<u32> {
        self.mapped_page
            .get(&(vaddr / PAGE_SIZE * PAGE_SIZE))
            .map(|page| {
                let offset = vaddr % PAGE_SIZE;
                let ins = page.lock()[offset..offset + 4].try_into().unwrap();
                u32::from_le_bytes(ins)
            })
    }
}
