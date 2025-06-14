//! 任务相关接口
//!
//! 本接口中包含 Task 结构体的定义和实现    
mod file;
mod info;
mod init;
mod mem;
mod signal;

use alloc::{collections::btree_map::BTreeMap, sync::Arc};
use common::{
    config::{CNODE_RADIX_BITS, DEFAULT_PARENT_EP, DEFAULT_SERVE_EP, PAGE_SIZE},
    page::PhysPage,
};
use core::{
    cmp,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use file::TaskFileInfo;
use info::TaskInfo;
use mem::TaskMemInfo;
use object::{File, Object, ObjectSection, ObjectSegment};
use sel4::{
    CapRights, Error, VmAttributes,
    init_thread::{self, slot},
};
use sel4_kit::slot_manager::LeafSlot;
use signal::TaskSignal;
use spin::Mutex;

use crate::{
    child_test::FutexTable,
    utils::obj::{alloc_cnode, alloc_page, alloc_pt, alloc_tcb, alloc_vspace},
};

/// Sel4Task 结构体
pub struct Sel4Task {
    /// 进程 ID
    pub pid: usize,
    /// 父进程 ID
    pub ppid: usize,
    /// 进程组 ID
    pub pgid: usize,
    /// 任务 ID (线程 ID)
    pub tid: usize,
    /// 进程控制块（Capability)
    pub tcb: sel4::cap::Tcb,
    /// 能力空间入口 (CSpace Root)
    pub cnode: sel4::cap::CNode,
    /// 地址空间 (Capability)
    pub vspace: sel4::cap::VSpace,
    /// 任务内存映射信息
    pub mem: Arc<Mutex<TaskMemInfo>>,
    /// 退出状态码
    pub exit: Option<i32>,
    /// Futex 表
    pub futex_table: Arc<Mutex<FutexTable>>,
    /// 信号信息
    pub signal: TaskSignal,
    /// The clear thread tid field
    ///
    /// See <https://manpages.debian.org/unstable/manpages-dev/set_tid_address.2.en.html#clear_child_tid>
    ///
    /// When the thread exits, the kernel clears the word at this address if it is not NULL.
    pub clear_child_tid: Option<usize>,
    /// 任务相关文件信息。
    pub file: TaskFileInfo,
    /// 定时器
    pub timer: Duration,
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

        if Arc::strong_count(&self.mem) == 1 {
            self.mem.lock().mapped_pt.iter().for_each(|cap| {
                root_cnode.absolute_cptr(*cap).revoke().unwrap();
                root_cnode.absolute_cptr(*cap).delete().unwrap();
            });
            self.mem.lock().mapped_page.values().for_each(|phys_page| {
                root_cnode.absolute_cptr(phys_page.cap()).revoke().unwrap();
                root_cnode.absolute_cptr(phys_page.cap()).delete().unwrap();
            });
        }
    }
}

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl Sel4Task {
    /// 创建一个新的任务
    pub fn new() -> Result<Self, sel4::Error> {
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
            .absolute_cptr_from_bits_with_depth(DEFAULT_PARENT_EP.bits(), CNODE_RADIX_BITS)
            .mint(
                &LeafSlot::from(DEFAULT_SERVE_EP).abs_cptr(),
                CapRights::all(),
                tid as u64,
            )?;

        Ok(Sel4Task {
            tid,
            pid: tid,
            pgid: 0,
            ppid: 1,
            tcb,
            cnode,
            vspace,
            futex_table: Arc::new(Mutex::new(BTreeMap::new())),
            mem: Arc::new(Mutex::new(TaskMemInfo::default())),
            signal: TaskSignal::default(),
            exit: None,
            clear_child_tid: None,
            file: TaskFileInfo::default(),
            info: TaskInfo::default(),
            timer: Duration::ZERO,
        })
    }

    /// 创建一个新的线程
    pub fn create_thread(&self) -> Result<Self, sel4::Error> {
        let tid = ID_COUNTER.fetch_add(1, Ordering::SeqCst) as usize;
        let tcb = alloc_tcb();
        let cnode = alloc_cnode(CNODE_RADIX_BITS);

        // 构建 CSpace 需要的结构
        cnode
            .absolute_cptr_from_bits_with_depth(1, CNODE_RADIX_BITS)
            .copy(&LeafSlot::from_cap(tcb).abs_cptr(), CapRights::all())
            .unwrap();

        // Copy EndPoint to child
        cnode
            .absolute_cptr_from_bits_with_depth(DEFAULT_PARENT_EP.bits(), CNODE_RADIX_BITS)
            .mint(
                &LeafSlot::from(DEFAULT_SERVE_EP).abs_cptr(),
                CapRights::all(),
                tid as u64,
            )?;

        Ok(Sel4Task {
            pid: self.pid,
            ppid: self.ppid,
            pgid: self.pgid,
            tid,
            tcb,
            cnode,
            vspace: self.vspace,
            mem: self.mem.clone(),
            exit: None,
            futex_table: self.futex_table.clone(),
            signal: TaskSignal::default(),
            clear_child_tid: None,
            file: self.file.clone(),
            info: self.info.clone(),
            timer: Duration::ZERO,
        })
    }

    /// 在当前任务的地址空间中找到最大可用的虚拟地址
    ///
    /// - `start` 从哪块内存开始
    /// - `size`  需要查找的内存块大小
    pub fn find_free_area(&self, start: usize, size: usize) -> usize {
        let mut last_addr = self.info.task_vm_end.max(start);
        for vaddr in self.mem.lock().mapped_page.keys() {
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
                    self.mem.lock().mapped_page.insert(vaddr, page);
                    return;
                }
                Err(Error::FailedLookup) => {
                    let pt_cap = alloc_pt();
                    pt_cap
                        .pt_map(self.vspace, vaddr, VmAttributes::DEFAULT)
                        .unwrap();
                    self.mem.lock().mapped_pt.push(pt_cap);
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
        if let Some(page) = self.mem.lock().mapped_page.remove(&vaddr) {
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
    pub fn load_elf(&mut self, file: &File<'_>) {
        // 加载程序到内存
        file.sections()
            .filter(|x| x.name() == Ok(".text"))
            .for_each(|sec| {
                #[cfg(target_arch = "aarch64")]
                {
                    const SVC_INST: u32 = 0xd4000001;
                    const ERR_INST: u32 = 0xdeadbeef;
                    let data = sec.data().unwrap();
                    let ptr = data.as_ptr() as *mut u32;
                    for i in 0..sec.size() as usize / size_of::<u32>() {
                        unsafe {
                            if ptr.add(i).read() == SVC_INST {
                                ptr.add(i).write_volatile(ERR_INST);
                            }
                        }
                    }
                }
                #[cfg(not(target_arch = "aarch64"))]
                log::warn!("Modify Syscall Instruction Not Supported For This Arch.");
            });
        file.segments().for_each(|seg| {
            let mut data = seg.data().unwrap();
            let mut vaddr = seg.address() as usize;
            let vaddr_end = vaddr + seg.size() as usize;
            log::debug!("load memory: {:#x} - {:#x}", vaddr, vaddr_end);

            while vaddr < vaddr_end {
                let voffset = vaddr % PAGE_SIZE;
                let page_cap = match self
                    .mem
                    .lock()
                    .mapped_page
                    .remove(&(vaddr / PAGE_SIZE * PAGE_SIZE))
                {
                    Some(page_cap) => {
                        page_cap.cap().frame_unmap().unwrap();
                        page_cap
                    }
                    None => PhysPage::new(alloc_page()),
                };

                // 将 elf 中特定段的内容写入对应的物理页中
                if !data.is_empty() {
                    let rsize = cmp::min(PAGE_SIZE - vaddr % PAGE_SIZE, data.len());
                    page_cap.lock()[voffset..voffset + rsize].copy_from_slice(&data[..rsize]);
                    data = &data[rsize..];
                }

                self.map_page(vaddr / PAGE_SIZE * PAGE_SIZE, page_cap);

                // Calculate offset
                vaddr += PAGE_SIZE - vaddr % PAGE_SIZE;
            }
        });

        // 配置程序最大的位置
        self.info.task_vm_end = file
            .sections()
            .fold(0, |acc, x| cmp::max(acc, x.address() + x.size()))
            .div_ceil(PAGE_SIZE as _) as usize
            * PAGE_SIZE;
    }
}
