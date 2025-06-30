//! 任务相关接口
//!
//! 本接口中包含 Task 结构体的定义和实现    
mod file;
mod info;
mod init;
mod mem;
mod pcb;
pub mod shm;
mod signal;

use alloc::{sync::Arc, vec::Vec};
use common::{
    config::{DEFAULT_PARENT_EP, DEFAULT_SERVE_EP, LINUX_APP_CNODE_RADIX_BITS, PAGE_SIZE},
    mem::CapMemSet,
    page::PhysPage,
    slot::{alloc_slot, recycle_slot},
};
use core::{
    cmp,
    sync::atomic::{AtomicU64, Ordering},
    task::Waker,
};
use file::TaskFileInfo;
use info::TaskInfo;
use libc_core::signal::SignalNum;
use mem::TaskMemInfo;
use object::{File, Object, ObjectSection, ObjectSegment};
use sel4::{
    CapRights, Error, VmAttributes,
    init_thread::{self, slot},
};
use sel4_kit::slot_manager::LeafSlot;
use signal::TaskSignal;
use spin::mutex::Mutex;
use zerocopy::IntoBytes;

use crate::{
    child_test::{FutexTable, TASK_MAP, futex_wake, wake_hangs},
    consts::task::VDSO_REGION_APP_ADDR,
    task::{pcb::ProcessControlBlock, shm::MapedSharedMemory},
    utils::obj::{alloc_untyped_unit, recycle_untyped_unit},
    vdso::get_vdso_caps,
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
    /// 资源内存分配器
    pub capset: Arc<Mutex<CapMemSet>>,
    /// 进程控制块（Capability)
    pub tcb: sel4::cap::Tcb,
    /// 能力空间入口 (CSpace Root)
    pub cnode: sel4::cap::CNode,
    /// 地址空间 (Capability)
    pub vspace: sel4::cap::VSpace,
    /// 任务内存映射信息
    pub mem: Arc<Mutex<TaskMemInfo>>,
    /// 共享内存信息
    pub shm: Arc<Mutex<Vec<Arc<MapedSharedMemory>>>>,
    /// 退出状态码
    pub exit: Mutex<Option<u32>>,
    /// Futex 表
    pub futex_table: Arc<Mutex<FutexTable>>,
    /// 信号信息
    pub signal: Mutex<TaskSignal>,
    /// The clear thread tid field
    ///
    /// See <https://manpages.debian.org/unstable/manpages-dev/set_tid_address.2.en.html#clear_child_tid>
    ///
    /// When the thread exits, the kernel clears the word at this address if it is not NULL.
    pub clear_child_tid: Mutex<usize>,
    /// 任务相关文件信息。
    pub file: TaskFileInfo,
    /// 任务初始信息，任务的初始信息记录在这里，方便进行初始化
    pub info: Mutex<TaskInfo>,
    /// 资源计数器，用于跟踪线程数量
    pub thread_counter: Mutex<Option<Arc<()>>>,
    /// 进程控制块
    pub pcb: Arc<ProcessControlBlock>,
    /// 异步 await 时存储的结构
    pub waker: Mutex<Option<(PollWakeEvent, Waker)>>,
}

/// 在 Poll 的时候唤醒协程的事件类型
pub enum PollWakeEvent {
    /// 被信号唤醒
    Signal(SignalNum),
    /// 被时钟唤醒
    Timer,
    /// 等待被唤醒
    Blocking,
}

impl Drop for Sel4Task {
    fn drop(&mut self) {
        // 释放文件描述符
        if Arc::strong_count(&self.file.file_ds) == 1 {
            for i in 0..=512 {
                self.file.file_ds.lock().remove(i);
            }
        }
    }
}

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl Sel4Task {
    /// 创建一个新的任务
    pub fn new() -> Result<Self, sel4::Error> {
        let tid = ID_COUNTER.fetch_add(1, Ordering::SeqCst) as usize;
        let mut capset = CapMemSet::new(Some(alloc_untyped_unit));

        let vspace = capset.alloc_vspace();
        let tcb = capset.alloc_tcb();
        let cnode = capset.alloc_cnode(LINUX_APP_CNODE_RADIX_BITS);
        slot::ASID_POOL.cap().asid_pool_assign(vspace).unwrap();

        // 构建 CSpace 需要的结构
        cnode
            .absolute_cptr_from_bits_with_depth(1, LINUX_APP_CNODE_RADIX_BITS)
            .copy(&LeafSlot::from_cap(tcb).abs_cptr(), CapRights::all())
            .unwrap();

        // Copy EndPoint to child
        cnode
            .absolute_cptr_from_bits_with_depth(
                DEFAULT_PARENT_EP.bits(),
                LINUX_APP_CNODE_RADIX_BITS,
            )
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
            shm: Arc::new(Mutex::new(Vec::new())),
            capset: Arc::new(Mutex::new(capset)),
            futex_table: Arc::new(Mutex::new(Vec::new())),
            mem: Arc::new(Mutex::new(TaskMemInfo::default())),
            signal: Mutex::new(TaskSignal::default()),
            exit: Mutex::new(None),
            clear_child_tid: Mutex::new(0),
            file: TaskFileInfo::default(),
            info: Mutex::new(TaskInfo::default()),
            thread_counter: Mutex::new(Some(Arc::new(()))),
            pcb: Arc::new(ProcessControlBlock::new()),
            waker: Mutex::new(None),
        })
    }

    /// 创建一个新的线程
    pub fn create_thread(&self) -> Result<Self, sel4::Error> {
        let capset = self.capset.clone();
        let tid = ID_COUNTER.fetch_add(1, Ordering::SeqCst) as usize;
        let tcb = capset.lock().alloc_tcb();
        let cnode = capset.lock().alloc_cnode(LINUX_APP_CNODE_RADIX_BITS);

        // 构建 CSpace 需要的结构
        cnode
            .absolute_cptr_from_bits_with_depth(1, LINUX_APP_CNODE_RADIX_BITS)
            .copy(&LeafSlot::from_cap(tcb).abs_cptr(), CapRights::all())
            .unwrap();

        // Copy EndPoint to child
        cnode
            .absolute_cptr_from_bits_with_depth(
                DEFAULT_PARENT_EP.bits(),
                LINUX_APP_CNODE_RADIX_BITS,
            )
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
            shm: self.shm.clone(),
            vspace: self.vspace,
            capset: self.capset.clone(),
            mem: self.mem.clone(),
            exit: Mutex::new(None),
            futex_table: self.futex_table.clone(),
            signal: Mutex::new(TaskSignal::default()),
            clear_child_tid: Mutex::new(0),
            file: self.file.clone(),
            info: Mutex::new(self.info.lock().clone()),
            thread_counter: Mutex::new(self.thread_counter.lock().clone()),
            pcb: self.pcb.clone(),
            waker: Mutex::new(None),
        })
    }

    /// 在当前任务的地址空间中找到最大可用的虚拟地址
    ///
    /// - `start` 从哪块内存开始
    /// - `size`  需要查找的内存块大小
    pub fn find_free_area(&self, start: usize, size: usize) -> usize {
        let mut last_addr = self.info.lock().task_vm_end.max(start);
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
    pub fn map_page(&self, vaddr: usize, page: PhysPage) {
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
                    let pt_cap = self.capset.lock().alloc_pt();
                    pt_cap
                        .pt_map(self.vspace, vaddr, VmAttributes::DEFAULT)
                        .unwrap();
                    self.mem.lock().mapped_pt.push(pt_cap);
                }
                _ => {
                    log::error!("map page to {:#x}", vaddr);
                    res.unwrap()
                }
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
            self.capset.lock().recycle_page(page.cap());
        }
    }

    /// 映射一个空白页到虚拟地址空间
    /// # 参数
    ///
    /// - `vaddr` 需要映射的虚拟地址，计算时会向下 4K 对齐
    pub fn map_blank_page(&self, mut vaddr: usize) -> PhysPage {
        vaddr = vaddr / PAGE_SIZE * PAGE_SIZE;
        let page_cap = PhysPage::new(self.capset.lock().alloc_page());
        self.map_page(vaddr, page_cap.clone());
        page_cap
    }

    /// 映射一个区域的内存
    ///
    /// - `start` 是起始地址
    /// - `end`   是结束地址
    ///
    /// 说明: 地址需要对齐到 0x1000
    pub fn map_region(&self, start: usize, end: usize) {
        assert!(end % 0x1000 == 0);
        assert!(start % 0x1000 == 0);

        for vaddr in (start..end).step_by(PAGE_SIZE) {
            self.map_blank_page(vaddr);
        }
    }

    /// 加载一个 elf 文件到当前任务的地址空间
    ///
    /// - `elf_data` 是 elf 文件的数据
    pub fn load_elf(&self, file: &File<'_>) {
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

            while vaddr < vaddr_end {
                let voffset = vaddr % PAGE_SIZE;
                let finded = self
                    .mem
                    .lock()
                    .mapped_page
                    .remove(&(vaddr / PAGE_SIZE * PAGE_SIZE));
                let page_cap = match finded {
                    Some(page_cap) => {
                        page_cap.cap().frame_unmap().unwrap();
                        page_cap
                    }
                    None => self.map_blank_page(vaddr),
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
        self.info.lock().task_vm_end = file
            .sections()
            .fold(0, |acc, x| cmp::max(acc, x.address() + x.size()))
            .div_ceil(PAGE_SIZE as _) as usize
            * PAGE_SIZE;

        get_vdso_caps().iter().enumerate().for_each(|(i, page)| {
            let new_slot = alloc_slot();
            new_slot
                .copy_from(&LeafSlot::from_cap(*page), CapRights::all())
                .unwrap();
            self.map_page(
                VDSO_REGION_APP_ADDR + i * PAGE_SIZE,
                PhysPage::new(new_slot.cap()),
            );
        });
    }

    /// 退出当前任务
    ///
    /// ## 参数
    /// - `code` 退出使用的 code
    pub fn exit_with(&self, code: u32) {
        *self.exit.lock() = Some(code);
        wake_hangs(self);
        let uaddr = *self.clear_child_tid.lock();
        if uaddr != 0 {
            self.write_bytes(uaddr, 0u32.as_bytes());
            futex_wake(self.futex_table.clone(), uaddr, 1);
        }
        if self.ppid != self.pid {
            if let Some(signal) = self.signal.lock().exit_sig {
                TASK_MAP
                    .lock()
                    .iter()
                    .find(|x| *x.0 == self.ppid as _)
                    .inspect(|parent| parent.1.add_signal(signal, self.tid));
            }
        }
        // 释放资源
        let root_cnode = init_thread::slot::CNODE.cap();
        root_cnode.absolute_cptr(self.tcb).revoke().unwrap();
        root_cnode.absolute_cptr(self.tcb).delete().unwrap();
        root_cnode.absolute_cptr(self.cnode).revoke().unwrap();
        root_cnode.absolute_cptr(self.cnode).delete().unwrap();
        recycle_slot(self.tcb.into());
        recycle_slot(self.cnode.into());

        if Arc::strong_count(self.thread_counter.lock().as_ref().unwrap()) == 1 {
            root_cnode.absolute_cptr(self.vspace).revoke().unwrap();
            root_cnode.absolute_cptr(self.vspace).delete().unwrap();
            recycle_slot(self.vspace.into());

            self.mem.lock().mapped_pt.iter().for_each(|cap| {
                root_cnode.absolute_cptr(*cap).revoke().unwrap();
                root_cnode.absolute_cptr(*cap).delete().unwrap();
                recycle_slot((*cap).into());
            });
            self.mem
                .lock()
                .mapped_page
                .iter()
                .for_each(|(_, phys_page)| {
                    root_cnode.absolute_cptr(phys_page.cap()).revoke().unwrap();
                    root_cnode.absolute_cptr(phys_page.cap()).delete().unwrap();
                    recycle_slot(phys_page.cap().into());
                });
            let mut capset = self.capset.lock();
            capset.release();
            capset.untyped_list().iter().for_each(|(untyped, _)| {
                root_cnode.absolute_cptr(*untyped).revoke().unwrap();
                recycle_untyped_unit(*untyped);
            });
        }
        // 释放文件描述符
        // if Arc::strong_count(&self.file.file_ds) == 1 {
        //     for i in 0..=512 {
        //         self.file.file_ds.lock().remove(i);
        //     }
        // }
        *self.thread_counter.lock() = None;
    }
}
