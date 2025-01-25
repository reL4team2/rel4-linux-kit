use crate::OBJ_ALLOCATOR;
use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use common::{page::PhysPage, USPACE_BASE};
use core::{
    cmp,
    sync::atomic::{AtomicU64, Ordering},
};
use crate_consts::{
    CNODE_RADIX_BITS, DEFAULT_PARENT_EP, DEFAULT_SERVE_EP, PAGE_SIZE, STACK_ALIGN_SIZE,
};
use memory_addr::MemoryAddr;
use sel4::{
    cap_type::PT,
    init_thread::{self, slot},
    CapRights, Error, VmAttributes,
};
use slot_manager::LeafSlot;
use xmas_elf::{program, ElfFile};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_camel_case_types, dead_code)]
pub enum AuxV {
    /// end of vector
    NULL = 0,
    /// entry should be ignored
    IGNORE = 1,
    /// file descriptor of program
    EXECFD = 2,
    /// program headers for program
    PHDR = 3,
    /// size of program header entry
    PHENT = 4,
    /// number of program headers
    PHNUM = 5,
    /// system page size
    PAGESZ = 6,
    /// base address of interpreter
    BASE = 7,
    /// flags
    FLAGS = 8,
    /// entry point of program
    ENTRY = 9,
    /// program is not ELF
    NOTELF = 10,
    /// real uid
    UID = 11,
    /// effective uid
    EUID = 12,
    /// real gid
    GID = 13,
    /// effective gid
    EGID = 14,
    /// string identifying CPU for optimizations
    PLATFORM = 15,
    /// arch dependent hints at CPU capabilities
    HWCAP = 16,
    /// frequency at which times() increments
    CLKTCK = 17,
    // values 18 through 22 are reserved
    DCACHEBSIZE = 19,
    /// secure mode boolean
    SECURE = 23,
    /// string identifying real platform, may differ from AT_PLATFORM
    BASE_PLATFORM = 24,
    /// address of 16 random bytes
    RANDOM = 25,
    /// extension of AT_HWCAP
    HWCAP2 = 26,
    /// filename of program
    EXECFN = 31,
}

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
        let vspace = OBJ_ALLOCATOR.lock().alloc_vspace();
        let tcb = OBJ_ALLOCATOR.lock().alloc_tcb();
        let cnode = OBJ_ALLOCATOR.lock().alloc_cnode(CNODE_RADIX_BITS);
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
                &LeafSlot::new(DEFAULT_SERVE_EP as _).abs_cptr(),
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
                    let pt_cap = OBJ_ALLOCATOR
                        .lock()
                        .allocate_and_retyped_fixed_sized::<PT>();
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

    pub fn map_stack(
        &mut self,
        entry_point: usize,
        start: usize,
        end: usize,
        args: &[&str],
    ) -> usize {
        assert!(end % 0x1000 == 0);
        assert!(start % 0x1000 == 0);
        let mut stack_ptr = end;

        for vaddr in (start..end).step_by(PAGE_SIZE) {
            let page_cap = PhysPage::new(OBJ_ALLOCATOR.lock().alloc_page());
            if vaddr == end - PAGE_SIZE {
                let mut page_writer = page_cap.lock();
                let args_ptr: Vec<_> = args
                    .iter()
                    .map(|arg| {
                        // TODO: set end bit was zeroed manually.
                        stack_ptr = (stack_ptr - arg.bytes().len()).align_down(STACK_ALIGN_SIZE);
                        page_writer.write_bytes(stack_ptr % PAGE_SIZE, arg.as_bytes());
                        stack_ptr
                    })
                    .collect();

                let mut push_num = |num: usize| {
                    stack_ptr = stack_ptr - core::mem::size_of::<usize>();
                    page_writer.write_usize(stack_ptr % PAGE_SIZE, num);
                    stack_ptr
                };

                let mut auxv = BTreeMap::new();
                auxv.insert(AuxV::EXECFN, args_ptr[0]);
                auxv.insert(AuxV::PAGESZ, PAGE_SIZE);
                auxv.insert(AuxV::ENTRY, entry_point);
                auxv.insert(AuxV::GID, 0);
                auxv.insert(AuxV::EGID, 0);
                auxv.insert(AuxV::UID, 0);
                auxv.insert(AuxV::EUID, 0);
                auxv.insert(AuxV::NULL, 0);

                // push auxiliary vector
                auxv.into_iter().for_each(|(key, v)| {
                    push_num(v);
                    push_num(key as usize);
                });
                // push environment
                push_num(0);
                // push args pointer
                push_num(0);
                args_ptr.iter().rev().for_each(|x| {
                    push_num(*x);
                });
                // push argv
                push_num(args_ptr.len());
            }
            self.map_page(vaddr, page_cap);
        }
        stack_ptr
    }

    pub fn load_elf(&mut self, elf_data: &[u8]) {
        let file = ElfFile::new(elf_data).expect("This is not a valid elf file");

        let mut mapped_page: BTreeMap<usize, PhysPage> = BTreeMap::new();

        // 从 elf 文件中读取数据
        file.program_iter()
            .filter(|ph| ph.get_type() == Ok(program::Type::Load))
            .for_each(|ph| {
                let mut offset = ph.offset() as usize;
                let mut vaddr = ph.virtual_addr() as usize;
                let end = offset + ph.file_size() as usize;
                let vaddr_end = vaddr + ph.mem_size() as usize;

                while vaddr < vaddr_end {
                    let page_cap = match mapped_page.remove(&(vaddr / PAGE_SIZE * PAGE_SIZE)) {
                        Some(page_cap) => {
                            page_cap.cap().frame_unmap().unwrap();
                            page_cap
                        }
                        None => PhysPage::new(OBJ_ALLOCATOR.lock().alloc_page()),
                    };

                    // 将 elf 中特定段的内容写入对应的物理页中
                    if offset < end {
                        let rsize = cmp::min(PAGE_SIZE - vaddr % PAGE_SIZE, end - offset);
                        page_cap.lock()[..rsize].copy_from_slice(&elf_data[offset..offset + rsize]);
                        offset += rsize;
                    }

                    self.map_page(vaddr / PAGE_SIZE * PAGE_SIZE, page_cap);
                    mapped_page.insert(vaddr / PAGE_SIZE * PAGE_SIZE, page_cap);

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
            let page_cap = PhysPage::new(OBJ_ALLOCATOR.lock().alloc_page());
            self.map_page(vaddr, page_cap);
        }
        value
    }
}
