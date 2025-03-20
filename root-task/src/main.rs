#![no_std]
#![no_main]
#![feature(never_type)]

extern crate alloc;

mod config;
mod handler;
mod task;
mod utils;

use ::config::{DEFAULT_CUSTOM_SLOT, PAGE_SIZE, VIRTIO_MMIO_ADDR};
use alloc::vec::Vec;
use common::*;
use config::TASK_FILES;
use include_bytes_aligned::include_bytes_aligned;
use page::PhysPage;
use sel4::{
    Cap, CapRights, ObjectBlueprintArm, UntypedDesc,
    cap::{LargePage, SmallPage, Untyped},
    cap_type::Endpoint,
    debug_println,
    init_thread::slot,
    with_ipc_buffer_mut,
};
use sel4_root_task::{Never, root_task};
use slot_manager::LeafSlot;
use spin::Mutex;
use task::*;

/// Object 分配器，可以用来申请 Capability
pub(crate) static OBJ_ALLOCATOR: Mutex<ObjectAllocator> = Mutex::new(ObjectAllocator::empty());

#[root_task(heap_size = 0x12_0000)]
fn main(bootinfo: &sel4::BootInfoPtr) -> sel4::Result<Never> {
    // 设置调试信息
    slot::TCB.cap().debug_name(b"root");
    init_log!(log::LevelFilter::Debug);

    // 初始化 untyped object
    let dev_untyped_start = bootinfo.untyped().start();
    let mem_untyped_start = dev_untyped_start + bootinfo.kernel_untyped_range().start;
    let mut mem_untypes = bootinfo.untyped_list()[bootinfo.kernel_untyped_range()]
        .iter()
        .enumerate()
        .map(|(idx, ud)| (Untyped::from_bits((mem_untyped_start + idx) as _), ud))
        .collect::<Vec<(Untyped, &UntypedDesc)>>();
    let device_untypes = bootinfo.untyped_list()[bootinfo.device_untyped_range()]
        .iter()
        .enumerate()
        .map(|(idx, ud)| (Untyped::from_bits((dev_untyped_start + idx) as _), ud))
        .collect::<Vec<(Untyped, &UntypedDesc)>>();
    mem_untypes.sort_by(|a, b| a.1.size_bits().cmp(&b.1.size_bits()));

    // 显示初始化信息
    utils::display_bootinfo(bootinfo, mem_untypes.as_slice(), device_untypes.as_slice());

    // 用最大块的内存初始化 Capability 分配器
    // 从 untyped object Retype 为特定的 object
    // TODO: 使用合适的 CSpace 边缘处理模式
    // 可能的做法是单独搞一个 ObjectAllocator 来分配 CSpace
    common::slot::init(bootinfo.empty().range().start..usize::MAX, None);
    OBJ_ALLOCATOR.lock().init(mem_untypes.pop().unwrap().0);

    // 重建 Capability 空间，构建为多级 CSpace
    rebuild_cspace();

    // Used for fault and normal IPC ( Reuse )
    let fault_ep = OBJ_ALLOCATOR.lock().alloc_endpoint();

    // 开始创建任务
    let mut tasks: Vec<Sel4Task> = Vec::new();
    for (id, task) in TASK_FILES.iter().enumerate() {
        tasks.push(build_kernel_thread(
            id,
            (fault_ep, tasks.len() as _),
            task.name,
            task.file,
        )?);
    }

    // 处理所有定义的任务
    TASK_FILES.iter().enumerate().for_each(|(t_idx, t)| {
        // 映射设备内存，应该保证映射到指定的物理地址上
        // 一般情况下映射的大小不会超过一个页
        // NOTICE: 目前不支持多个人物共享一个物理页表的情况
        // TODO: 实现申请指定物理内存到映射到特定物理地址的操作
        for (vaddr, paddr, _size) in t.mem {
            let (blk_device_untyped_cap, _) = device_untypes
                .iter()
                .find(|(_, desc)| {
                    (desc.paddr()..(desc.paddr() + (1 << desc.size_bits()))).contains(paddr)
                })
                .expect("[RootTask] can't find device memory");
            let leaf_slot = OBJ_ALLOCATOR.lock().allocate_slot();
            let blk_device_frame_cap = LargePage::from_bits(leaf_slot.raw() as _);

            blk_device_untyped_cap
                .untyped_retype(
                    &ObjectBlueprintArm::LargePage.into(),
                    &leaf_slot.cnode_abs_cptr(),
                    leaf_slot.offset_of_cnode(),
                    1,
                )
                .unwrap();

            assert!(blk_device_frame_cap.frame_get_address().unwrap() < VIRTIO_MMIO_ADDR);

            tasks[t_idx].map_large_page(*vaddr, blk_device_frame_cap);
        }

        // 映射 DMA 内存，应该随机分配任意内存即可
        for (start, size) in t.dma {
            // 申请多个页表
            // TODO: 检查页表是否连续
            let pages_cap = OBJ_ALLOCATOR.lock().alloc_pages(size / PAGE_SIZE);

            // 映射多个页表
            pages_cap.into_iter().enumerate().for_each(|(i, page)| {
                debug_println!(
                    "[RootTask] Mapping DMA {:#x} -> {:#x}",
                    start + i * PAGE_SIZE,
                    page.frame_get_address().unwrap()
                );
                tasks[t_idx].map_page(start + i * PAGE_SIZE, PhysPage::new(page));
            });
        }

        // FIXME: 将分配内存的逻辑写成一个通用的逻辑
        if t.name == "kernel-thread" {
            let (mem_cap, _) = mem_untypes.pop().unwrap();
            tasks[t_idx]
                .cnode
                .absolute_cptr_from_bits_with_depth(DEFAULT_CUSTOM_SLOT, 64)
                .copy(&LeafSlot::from_cap(mem_cap).abs_cptr(), CapRights::all())
                .unwrap()
        }
    });

    run_tasks(&tasks);
    let mut root_task_handler = RootTaskHandler {
        tasks,
        fault_ep,
        badge: 0,
        channels: Vec::new(),
    };
    with_ipc_buffer_mut(|ib| root_task_handler.waiting_and_handle(ib))
}

pub struct RootTaskHandler {
    tasks: Vec<Sel4Task>,
    fault_ep: Cap<Endpoint>,
    badge: u64,
    channels: Vec<(usize, Vec<SmallPage>)>,
}
