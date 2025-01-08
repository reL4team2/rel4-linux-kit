use crate::{abs_cptr, GRANULE_SIZE, OBJ_ALLOCATOR};
use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use common::{footprint, map_image, map_intermediate_translation_tables};
use core::ops::DerefMut;
use crate_consts::CNODE_RADIX_BITS;
use object::{File, Object};
use sel4::{
    cap::{Endpoint, Null},
    cap_type::{self, CNode, SmallPage, Tcb, PT},
    debug_println,
    init_thread::slot,
    CNodeCapData, CapRights,
};
use task_helper::{Sel4TaskHelper, TaskHelperTrait};
use xmas_elf::ElfFile;

pub struct TaskImpl;
pub type Sel4Task = Sel4TaskHelper<TaskImpl>;

impl TaskHelperTrait<Sel4TaskHelper<Self>> for TaskImpl {
    const DEFAULT_STACK_TOP: usize = 0x1_0000_0000;

    fn allocate_pt(_task: &mut Self::Task) -> sel4::cap::PT {
        OBJ_ALLOCATOR
            .lock()
            .allocate_and_retyped_fixed_sized::<PT>()
    }

    fn allocate_page(_task: &mut Self::Task) -> sel4::cap::SmallPage {
        OBJ_ALLOCATOR
            .lock()
            .allocate_and_retyped_fixed_sized::<SmallPage>()
    }
}

pub fn rebuild_cspace() {
    let cnode = OBJ_ALLOCATOR
        .lock()
        .allocate_variable_sized_origin::<CNode>(CNODE_RADIX_BITS);
    cnode
        .absolute_cptr_from_bits_with_depth(0, CNODE_RADIX_BITS)
        .mint(
            &slot::CNODE.cap().absolute_cptr(slot::CNODE.cap()),
            CapRights::all(),
            CNodeCapData::skip(0).into_word(),
        )
        .unwrap();

    // load
    slot::CNODE
        .cap()
        .absolute_cptr(Null::from_bits(0))
        .mutate(
            &slot::CNODE.cap().absolute_cptr(slot::CNODE.cap()),
            CNodeCapData::skip_high_bits(CNODE_RADIX_BITS).into_word(),
        )
        .unwrap();

    sel4::cap::CNode::from_bits(0)
        .absolute_cptr(slot::CNODE.cap())
        .mint(
            &sel4::cap::CNode::from_bits(0).absolute_cptr(cnode),
            CapRights::all(),
            CNodeCapData::skip_high_bits(CNODE_RADIX_BITS * 2).into_word(),
        )
        .unwrap();

    slot::CNODE
        .cap()
        .absolute_cptr(Null::from_bits(0))
        .delete()
        .unwrap();

    slot::TCB
        .cap()
        .tcb_set_space(
            Null::from_bits(0).cptr(),
            cnode,
            CNodeCapData::skip_high_bits(2 * CNODE_RADIX_BITS),
            slot::VSPACE.cap(),
        )
        .unwrap();
}

pub fn build_kernel_thread(
    fault_ep: (Endpoint, u64),
    thread_name: &str,
    file_data: &[u8],
    free_page_addr: usize,
) -> sel4::Result<Sel4Task> {
    // make 新线程的虚拟地址空间
    let cnode = OBJ_ALLOCATOR
        .lock()
        .allocate_and_retyped_variable_sized::<CNode>(CNODE_RADIX_BITS);
    let mut mapped_page = BTreeMap::new();
    let (vspace, ipc_buffer_addr, ipc_buffer_cap) = make_child_vspace(
        cnode,
        &mut mapped_page,
        &File::parse(file_data).unwrap(),
        slot::VSPACE.cap(),
        free_page_addr,
        slot::ASID_POOL.cap(),
    );

    let tcb = OBJ_ALLOCATOR
        .lock()
        .allocate_and_retyped_fixed_sized::<Tcb>();

    let srv_ep = OBJ_ALLOCATOR
        .lock()
        .allocate_and_retyped_fixed_sized::<cap_type::Endpoint>();

    let mut task = Sel4Task::new(
        tcb,
        cnode,
        fault_ep.0,
        srv_ep,
        vspace,
        mapped_page,
        fault_ep.1,
    );

    // Configure TCB
    task.configure(2 * CNODE_RADIX_BITS, ipc_buffer_addr, ipc_buffer_cap)?;

    // Map stack for the task.
    task.map_stack(10);

    // set task priority and max control priority
    task.tcb.tcb_set_sched_params(slot::TCB.cap(), 255, 255)?;

    task.tcb.debug_name(thread_name.as_bytes());
    task.set_name(thread_name);

    task.with_context(&ElfFile::new(file_data).expect("parse elf error"));

    debug_println!(
        "[RootTask] Task: {} created. cnode: {:?}, vspace: {:?}",
        thread_name,
        task.cnode,
        task.vspace
    );

    Ok(task)
}

pub fn run_tasks(tasks: &Vec<Sel4Task>) {
    tasks.iter().for_each(Sel4Task::run)
}

/// 创建一个新的虚拟地址空间
/// # Parameters
/// - `image`: ELF 文件
/// - `caller_vspace`: root-task 的虚拟地址空间
/// - `free_page_addr`: 空闲页的地址
/// - `asid_pool`: ASID 池
/// # Returns
/// - `sel4::cap::VSpace`: 新的虚拟地址空间
/// - `usize`: IPC buffer 的地址
/// - `sel4::cap::Granule`: IPC buffer 的 cap
pub(crate) fn make_child_vspace<'a>(
    cnode: sel4::cap::CNode,
    mapped_page: &mut BTreeMap<usize, sel4::cap::Granule>,
    image: &'a impl Object<'a>,
    caller_vspace: sel4::cap::VSpace,
    free_page_addr: usize,
    asid_pool: sel4::cap::AsidPool,
) -> (sel4::cap::VSpace, usize, sel4::cap::Granule) {
    let inner_cnode = OBJ_ALLOCATOR
        .lock()
        .allocate_and_retyped_variable_sized::<CNode>(CNODE_RADIX_BITS);
    let mut allocator = OBJ_ALLOCATOR.lock();
    let allocator = allocator.deref_mut();
    let child_vspace = allocator.allocate_and_retyped_fixed_sized::<sel4::cap_type::VSpace>();
    // Build 2 level CSpace.
    // | unused (40 bits) | Level1 (12 bits) | Level0 (12 bits) |
    cnode
        .absolute_cptr_from_bits_with_depth(0, CNODE_RADIX_BITS)
        .mutate(
            &abs_cptr(inner_cnode),
            CNodeCapData::skip(0).into_word() as _,
        )
        .unwrap();
    abs_cptr(Null::from_bits(0))
        .mutate(
            &abs_cptr(cnode),
            CNodeCapData::skip_high_bits(2 * CNODE_RADIX_BITS).into_word() as _,
        )
        .unwrap();
    abs_cptr(cnode)
        .mutate(
            &abs_cptr(Null::from_bits(0)),
            CNodeCapData::skip_high_bits(2 * CNODE_RADIX_BITS).into_word() as _,
        )
        .unwrap();
    asid_pool.asid_pool_assign(child_vspace).unwrap();

    let image_footprint = footprint(image);

    // 将ELF的虚地址空间 map 到页表中，但不分配物理页
    map_intermediate_translation_tables(
        allocator,
        child_vspace,
        image_footprint.start..(image_footprint.end + GRANULE_SIZE),
    );

    // 将ELF的虚地址 map 到物理页
    map_image(
        allocator,
        mapped_page,
        child_vspace,
        image_footprint.clone(),
        image,
        caller_vspace,
        free_page_addr,
    );

    // make ipc buffer
    let ipc_buffer_addr = image_footprint.end;
    let ipc_buffer_cap = allocator.allocate_and_retyped_fixed_sized::<sel4::cap_type::Granule>();
    ipc_buffer_cap
        .frame_map(
            child_vspace,
            ipc_buffer_addr,
            sel4::CapRights::all(),
            sel4::VmAttributes::default(),
        )
        .unwrap();
    mapped_page.insert(ipc_buffer_addr, ipc_buffer_cap);

    (child_vspace, ipc_buffer_addr, ipc_buffer_cap)
}