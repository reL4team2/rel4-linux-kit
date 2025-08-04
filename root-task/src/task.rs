use crate::{
    OBJ_ALLOCATOR,
    utils::{footprint, map_image, map_intermediate_translation_tables},
};
use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use common::{
    config::{
        self, CNODE_RADIX_BITS, DEFAULT_PARENT_EP, DEFAULT_SERVE_EP, DEFAULT_THREAD_NOTIFICATION,
        PAGE_SIZE, STACK_ALIGN_SIZE,
    },
    page::PhysPage,
};
use object::{File, Object};
use sel4::{
    AbsoluteCPtr, CNodeCapData, CPtr, CapRights, Error, HasCPtrWithDepth, UserContext,
    VmAttributes as VMAttributes,
    cap::{self, Endpoint, Granule, Notification, SmallPage},
    debug_println,
    init_thread::{self, slot},
};
use sel4_kit::slot_manager::LeafSlot;
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

/// sel4 任务类型的助手函数，帮助快速实现一个sel
pub struct Sel4Task {
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
    /// 已经映射的大页
    pub mapped_large_page: BTreeMap<usize, cap::LargePage>,
    /// 栈底
    pub stack_bottom: usize,
}

impl Sel4Task {
    /// 创建一个新的 Task
    pub fn new(
        tcb: cap::Tcb,
        cnode: cap::CNode,
        fault_ep: cap::Endpoint,
        srv_ep: cap::Endpoint,
        vspace: cap::VSpace,
        mapped_page: BTreeMap<usize, PhysPage>,
        mapped_large_page: BTreeMap<usize, cap::LargePage>,
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
            mapped_large_page,
            stack_bottom: config::SERVICE_BOOT_STACK_TOP,
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
                    let pt_cap = OBJ_ALLOCATOR.alloc_pt();
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
                    self.mapped_large_page.insert(vaddr, page);
                    return;
                }
                // Map page tbale if the fault is Error::FailedLookup
                // (It's indicates that here was not a page table).
                Err(Error::FailedLookup) => {
                    let pt_cap = OBJ_ALLOCATOR.alloc_pt();
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
            let page_cap = PhysPage::new(OBJ_ALLOCATOR.alloc_page());
            self.map_page(self.stack_bottom + i * PAGE_SIZE, page_cap);
        }
    }

    /// Get the the absolute cptr related to task's cnode through cptr_bits.
    pub fn abs_cptr(&self, cptr: CPtr) -> AbsoluteCPtr {
        self.cnode.absolute_cptr(cptr)
    }

    /// 为当前的任务中创建一个新的线程
    #[allow(dead_code)]
    pub fn clone_thread(&self, tcb: sel4::cap::Tcb, srv_ep: cap::Endpoint) -> Self {
        Self {
            name: String::new(),
            tcb,
            srv_ep,
            cnode: self.cnode,
            vspace: self.vspace,
            mapped_pt: self.mapped_pt.clone(),
            mapped_page: self.mapped_page.clone(),
            mapped_large_page: self.mapped_large_page.clone(),
            stack_bottom: self.stack_bottom,
        }
    }

    /// FIXME: 创建一个上下文结构
    pub fn with_context(&self, image: &ElfFile) {
        let mut user_context = sel4::UserContext::default();
        *user_context.pc_mut() = image.header.pt2.entry_point();

        *user_context.sp_mut() = (config::SERVICE_BOOT_STACK_TOP - STACK_ALIGN_SIZE) as _;
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

pub fn build_kernel_thread(
    id: usize,
    fault_ep: (Endpoint, u64),
    thread_name: &str,
    file_data: &[u8],
) -> sel4::Result<Sel4Task> {
    // make 新线程的虚拟地址空间
    let cnode = OBJ_ALLOCATOR.alloc_cnode(CNODE_RADIX_BITS);
    let mut mapped_page = BTreeMap::new();
    let (vspace, ipc_buffer_addr, ipc_buffer_cap) = make_child_vspace(
        cnode,
        &mut mapped_page,
        &File::parse(file_data).unwrap(),
        slot::ASID_POOL.cap(),
    );

    let tcb = OBJ_ALLOCATOR.alloc_tcb();
    let srv_ep = OBJ_ALLOCATOR.alloc_endpoint();

    let mut task = Sel4Task::new(
        tcb,
        cnode,
        fault_ep.0,
        srv_ep,
        vspace,
        mapped_page,
        BTreeMap::new(),
        fault_ep.1,
    );

    // Configure TCB
    task.configure(2 * CNODE_RADIX_BITS, ipc_buffer_addr, ipc_buffer_cap)?;

    // Map stack for the task.
    task.map_stack(config::SERVICE_BOOT_STACK_SIZE.div_ceil(PAGE_SIZE));

    // set task priority and max control priority
    task.tcb.tcb_set_sched_params(slot::TCB.cap(), 255, 255)?;

    task.tcb.debug_name(thread_name.as_bytes());
    task.set_name(thread_name);

    task.with_context(&ElfFile::new(file_data).expect("parse elf error"));

    debug_println!(
        "[RootTask] Spawn {} {}. CNode: {:?}, VSpace: {:?}",
        thread_name,
        id,
        task.cnode,
        task.vspace
    );

    Ok(task)
}

pub fn run_tasks(tasks: &[Sel4Task]) {
    tasks.iter().for_each(Sel4Task::run)
}

/// 创建一个新的虚拟地址空间
///
/// # Parameters
/// - `image`: ELF 文件
/// - `caller_vspace`: root-task 的虚拟地址空间
/// - `free_page_addr`: 空闲页的地址
/// - `asid_pool`: ASID 池
///
/// # Returns
/// - `sel4::cap::VSpace`: 新的虚拟地址空间
/// - `usize`: IPC buffer 的地址
/// - `sel4::cap::Granule`: IPC buffer 的 cap
pub(crate) fn make_child_vspace<'a>(
    cnode: sel4::cap::CNode,
    mapped_page: &mut BTreeMap<usize, PhysPage>,
    image: &'a impl Object<'a>,
    asid_pool: sel4::cap::AsidPool,
) -> (sel4::cap::VSpace, usize, SmallPage) {
    let inner_cnode = OBJ_ALLOCATOR.alloc_cnode(CNODE_RADIX_BITS);
    let allocator = &OBJ_ALLOCATOR;
    let child_vspace = allocator.allocate_and_retyped_fixed_sized::<sel4::cap_type::VSpace>();
    // Build 2 level CSpace.
    // | unused (40 bits) | Level1 (12 bits) | Level0 (12 bits) |
    cnode
        .absolute_cptr_from_bits_with_depth(0, CNODE_RADIX_BITS)
        .mutate(
            &LeafSlot::from_cap(inner_cnode).abs_cptr(),
            CNodeCapData::skip(0).into_word() as _,
        )
        .unwrap();
    LeafSlot::new(0)
        .abs_cptr()
        .mutate(
            &LeafSlot::new(cnode.bits() as _).abs_cptr(),
            CNodeCapData::skip_high_bits(2 * CNODE_RADIX_BITS).into_word() as _,
        )
        .unwrap();
    LeafSlot::new(cnode.bits() as _)
        .abs_cptr()
        .mutate(
            &LeafSlot::new(0).abs_cptr(),
            CNodeCapData::skip_high_bits(2 * CNODE_RADIX_BITS).into_word() as _,
        )
        .unwrap();
    asid_pool.asid_pool_assign(child_vspace).unwrap();

    let image_footprint = footprint(image);

    // 将ELF的虚地址空间 map 到页表中，但不分配物理页
    map_intermediate_translation_tables(
        allocator,
        child_vspace,
        image_footprint.start..(image_footprint.end + PAGE_SIZE),
    );

    // 将ELF的虚地址 map 到物理页
    map_image(
        allocator,
        mapped_page,
        child_vspace,
        image_footprint.clone(),
        image,
    );

    // make ipc buffer
    let ipc_buffer_addr = image_footprint.end;
    let ipc_buffer_cap = allocator.alloc_page();
    ipc_buffer_cap
        .frame_map(
            child_vspace,
            ipc_buffer_addr,
            sel4::CapRights::all(),
            sel4::VmAttributes::default(),
        )
        .unwrap();
    mapped_page.insert(ipc_buffer_addr, PhysPage::new(ipc_buffer_cap));

    (child_vspace, ipc_buffer_addr, ipc_buffer_cap)
}
