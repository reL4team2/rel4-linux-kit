//! task-helper 提供一套快速构建 sel4 task 的工具
//!
//! 此 crate 中内置了一些设定，帮助构建一套通用的体系来服务于 Service 的构建

#![no_std]
#![deny(missing_docs)]
#![feature(associated_type_defaults)]

extern crate alloc;

use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use common::{
    consts::{DEFAULT_PARENT_EP, DEFAULT_SERVE_EP, DEFAULT_THREAD_NOTIFICATION},
    page::PhysPage,
};
use config::{PAGE_SIZE, STACK_ALIGN_SIZE};
use core::marker::PhantomData;
use sel4::{
    AbsoluteCPtr, CNodeCapData, CPtr, CapRights, Error, HasCPtrWithDepth,
    VmAttributes as VMAttributes,
    cap::{self, Granule, Notification},
    init_thread::{self, slot},
};
use sel4_sync::{MutexSyncOpsWithNotification, lock_api::Mutex};
use xmas_elf::{ElfFile, program};

/// Thread Notifications implementation
pub struct ThreadNotification;

/// Implement [MutexSyncOpsWithNotification] for [ThreadNotification]
/// Get the notification in the specificed thread slot.
impl MutexSyncOpsWithNotification for ThreadNotification {
    fn notification(&self) -> Notification {
        Notification::from_bits(DEFAULT_THREAD_NOTIFICATION)
    }
}

/// Mutex with Notification.
// pub type NotiMutex<T> = Mutex<GenericRawMutex<ThreadNotification>, T>;
pub type NotiMutex<T> = Mutex<spin::Mutex<()>, T>;

/// sel4 的任务的抽象，实现这个 Trait 能够帮助你快速实现一个 sel4 的任务
pub trait TaskHelperTrait<V> {
    /// 任务类型
    type Task = V;
    /// 默认的栈顶
    const DEFAULT_STACK_TOP: usize;
    /// 申请一个页表
    fn allocate_pt(task: &mut V) -> sel4::cap::PT;
    /// 申请一个页
    fn allocate_page(task: &mut V) -> sel4::cap::Granule;
}

/// sel4 任务类型的助手函数，帮助快速实现一个sel
pub struct Sel4TaskHelper<H: TaskHelperTrait<Self>> {
    /// 任务名称
    pub name: String,
    /// TCB Capability
    pub tcb: cap::Tcb,
    /// CSpace 的 Root Capability
    /// 有点类似于页表的入口
    pub cnode: cap::CNode,
    /// 地址空间 Capability
    pub vspace: cap::VSpace,
    /// 服务端口，向外提供服务
    pub srv_ep: cap::Endpoint,
    /// 已经映射的页表
    pub mapped_pt: Arc<NotiMutex<Vec<cap::PT>>>,
    /// 已经映射的页
    pub mapped_page: BTreeMap<usize, PhysPage>,
    /// 栈底
    pub stack_bottom: usize,
    /// 幽灵类型
    pub phantom: PhantomData<H>,
}

impl<H: TaskHelperTrait<Self>> Sel4TaskHelper<H> {
    /// 创建一个新的 Task
    pub fn new(
        tcb: cap::Tcb,
        cnode: cap::CNode,
        fault_ep: cap::Endpoint,
        srv_ep: cap::Endpoint,
        vspace: cap::VSpace,
        mapped_page: BTreeMap<usize, PhysPage>,
        badge: u64,
    ) -> Self {
        let task = Self {
            name: String::new(),
            tcb,
            cnode,
            vspace,
            srv_ep,
            mapped_pt: Arc::new(Mutex::new(Vec::new())),
            mapped_page,
            stack_bottom: H::DEFAULT_STACK_TOP,
            phantom: PhantomData,
        };

        // Move Fault EP to child process
        task.abs_cptr(DEFAULT_PARENT_EP.cptr())
            .mint(&cnode_relative(fault_ep), CapRights::all(), badge)
            .unwrap();

        // Move SRV EP to child process
        task.abs_cptr(DEFAULT_SERVE_EP.cptr())
            .copy(&cnode_relative(srv_ep), CapRights::all())
            .unwrap();

        // Copy ASIDPool to the task, children can assign another children.
        task.abs_cptr(init_thread::slot::ASID_POOL.cptr())
            .copy(&cnode_relative(slot::ASID_POOL.cap()), CapRights::all())
            .unwrap();

        // Copy ASIDControl to the task, children can assign another children.
        task.abs_cptr(init_thread::slot::ASID_CONTROL.cptr())
            .copy(&cnode_relative(slot::ASID_CONTROL.cap()), CapRights::all())
            .unwrap();

        task
    }

    /// 映射一个页表 [sel4::cap::Granule] 到指定的虚拟地址
    pub fn map_page(&mut self, vaddr: usize, page: PhysPage) {
        assert_eq!(vaddr % PAGE_SIZE, 0);
        for _ in 0..sel4::vspace_levels::NUM_LEVELS {
            let res: core::result::Result<(), sel4::Error> = page.cap().frame_map(
                self.vspace,
                vaddr as _,
                CapRights::all(),
                VMAttributes::DEFAULT,
            );
            match res {
                Ok(_) => {
                    self.mapped_page.insert(vaddr, page);
                    return;
                }
                // Map page tbale if the fault is Error::FailedLookup
                // (It's indicates that here was not a page table).
                Err(Error::FailedLookup) => {
                    let pt_cap = H::allocate_pt(self);
                    pt_cap
                        .pt_map(self.vspace, vaddr, VMAttributes::DEFAULT)
                        .unwrap();
                    self.mapped_pt.lock().push(pt_cap);
                }
                _ => res.unwrap(),
            }
        }
        unreachable!("Failed to map page!")
    }

    /// 映射一个大页 [sel4::cap::LargePage] 到指定的虚拟地址
    pub fn map_large_page(&mut self, vaddr: usize, page: sel4::cap::LargePage) {
        assert_eq!(vaddr % PAGE_SIZE, 0);
        for _ in 0..sel4::vspace_levels::NUM_LEVELS {
            let res: core::result::Result<(), sel4::Error> = page.frame_map(
                self.vspace,
                vaddr as _,
                CapRights::all(),
                VMAttributes::DEFAULT,
            );
            match res {
                Ok(_) => {
                    log::debug!("[TaskHelper] map device memory success");
                    // FIXME: Record The Mapped Page.
                    return;
                }
                // Map page tbale if the fault is Error::FailedLookup
                // (It's indicates that here was not a page table).
                Err(Error::FailedLookup) => {
                    let pt_cap = H::allocate_pt(self);
                    pt_cap
                        .pt_map(self.vspace, vaddr, VMAttributes::DEFAULT)
                        .unwrap();
                    self.mapped_pt.lock().push(pt_cap);
                }
                _ => res.unwrap(),
            }
        }
        unreachable!("Failed to map page!")
    }

    /// Configure task with setting CNode, Tcb and VSpace Cap
    pub fn configure(
        &mut self,
        radix_bits: usize,
        ipc_buffer_addr: usize,
        ipc_buffer_cap: Granule,
    ) -> Result<(), Error> {
        // Move cap rights to task
        self.abs_cptr(init_thread::slot::CNODE.cptr())
            .mint(
                &cnode_relative(self.cnode),
                CapRights::all(),
                CNodeCapData::skip_high_bits(radix_bits).into_word(),
            )
            .unwrap();

        // Copy tcb to task
        self.abs_cptr(init_thread::slot::TCB.cptr())
            .copy(&cnode_relative(self.tcb), CapRights::all())
            .unwrap();

        // Copy vspace to task
        self.abs_cptr(init_thread::slot::VSPACE.cptr())
            .copy(&cnode_relative(self.vspace), CapRights::all())
            .unwrap();

        self.tcb.tcb_configure(
            DEFAULT_PARENT_EP.cptr(),
            self.cnode,
            CNodeCapData::skip_high_bits(radix_bits),
            self.vspace,
            ipc_buffer_addr as _,
            ipc_buffer_cap,
        )
    }

    /// 映射指定数量的页到栈底
    pub fn map_stack(&mut self, page_count: usize) {
        self.stack_bottom -= page_count * PAGE_SIZE;
        for i in 0..page_count {
            let page_cap = PhysPage::new(H::allocate_page(self));
            self.map_page(self.stack_bottom + i * PAGE_SIZE, page_cap);
        }
    }

    /// Get the the absolute cptr related to task's cnode through cptr_bits.
    pub fn abs_cptr(&self, cptr: CPtr) -> AbsoluteCPtr {
        self.cnode.absolute_cptr(cptr)
    }
    /// Clone a new thread from the current thread.
    pub fn clone_thread(&self, tcb: sel4::cap::Tcb, srv_ep: cap::Endpoint) -> Self {
        Self {
            name: String::new(),
            tcb,
            srv_ep,
            cnode: self.cnode,
            vspace: self.vspace,
            mapped_pt: self.mapped_pt.clone(),
            mapped_page: self.mapped_page.clone(),
            stack_bottom: self.stack_bottom,
            phantom: PhantomData,
        }
    }

    /// FIXME: 创建一个上下文结构
    pub fn with_context(&self, image: &ElfFile) {
        let mut user_context = sel4::UserContext::default();
        *user_context.pc_mut() = image.header.pt2.entry_point();

        *user_context.sp_mut() = (H::DEFAULT_STACK_TOP - STACK_ALIGN_SIZE) as _;
        user_context.inner_mut().tpidr_el0 = image
            .program_iter()
            .find(|x| x.get_type() == Ok(program::Type::Tls))
            .map(|x| x.virtual_addr())
            .unwrap_or(0);

        self.tcb
            .tcb_write_all_registers(false, &mut user_context)
            .expect("can't write pc reg to tcb")
    }

    /// 设置任务名称
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    /// 将任务设置为运行状态
    pub fn run(&self) {
        self.tcb.tcb_resume().unwrap();
    }
}

/// Get the the absolute cptr related to current cnode through cptr_bits.
pub fn cnode_relative<T: HasCPtrWithDepth>(path: T) -> AbsoluteCPtr {
    init_thread::slot::CNODE.cap().absolute_cptr(path)
}
