#![no_std]
#![no_main]
#![feature(never_type)]

extern crate alloc;

mod config;
mod task;
mod utils;

use alloc::vec::Vec;
use common::services::IpcBufferRW;
use common::*;
use config::TASK_FILES;
use crate_consts::*;
use include_bytes_aligned::include_bytes_aligned;
use page::PhysPage;
use sel4::{
    cap::{LargePage, Untyped},
    cap_type::Endpoint,
    debug_println,
    init_thread::slot,
    with_ipc_buffer, with_ipc_buffer_mut, Cap, CapRights, Fault, IpcBuffer, MessageInfoBuilder,
    ObjectBlueprintArm, UntypedDesc,
};
use sel4_root_task::{root_task, Never};
use services::root::RootEvent;
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
    common::slot::init(bootinfo.empty().range());
    OBJ_ALLOCATOR.lock().init(mem_untypes.pop().unwrap().0);

    // 重建 Capability 空间，构建为多级 CSpace
    rebuild_cspace();

    // Used for fault and normal IPC ( Reuse )
    let fault_ep = OBJ_ALLOCATOR.lock().alloc_endpoint();

    // 开始创建任务
    let mut tasks: Vec<Sel4Task> = Vec::new();
    for task in TASK_FILES.iter() {
        tasks.push(build_kernel_thread(
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
    with_ipc_buffer_mut(|ib| handle_ep(tasks.as_mut_slice(), fault_ep, ib))
}

/// Handle the end point.
fn handle_ep(tasks: &mut [Sel4Task], fault_ep: Cap<Endpoint>, ib: &mut IpcBuffer) -> ! {
    let rev_msg = MessageInfoBuilder::default();
    let swap_slot = OBJ_ALLOCATOR.lock().allocate_slot();
    loop {
        let (message, badge) = fault_ep.recv(());
        let msg_label = RootEvent::from(message.label());

        match msg_label {
            RootEvent::Ping => sel4::reply(ib, rev_msg.build()),
            RootEvent::TranslateAddr => {
                let mut off = 0;
                let addr = usize::read_buffer(ib, &mut off);

                let phys_addr = tasks[badge as usize]
                    .mapped_page
                    .get(&(addr & !0xfff))
                    .map(|x| x.addr())
                    .unwrap();

                ib.msg_regs_mut()[0] = (phys_addr + addr % 0x1000) as _;
                sel4::reply(ib, rev_msg.length(off).build());
            }
            RootEvent::FindService => {
                let name = <&str>::read_buffer(ib, &mut 0);
                let task = tasks.iter().find(|task| task.name == name);
                let msg = match task {
                    Some(task) => {
                        LeafSlot::from(task.srv_ep)
                            .mint_to(swap_slot, CapRights::all(), badge as _)
                            .unwrap();
                        ib.caps_or_badges_mut()[0] = swap_slot.raw() as _;
                        let msg = rev_msg.extra_caps(1).build();
                        msg
                    }
                    // 发生错误时返回值 不为 -1
                    None => rev_msg.label(1).build(),
                };
                sel4::reply(ib, msg);
                let _ = swap_slot.delete();
            }
            // Allocate a irq handler capability
            // Transfer it to the requested service
            RootEvent::RegisterIRQ => {
                let irq = ib.msg_regs()[0];
                let dst_slot = LeafSlot::new(0);
                slot::IRQ_CONTROL
                    .cap()
                    .irq_control_get(irq, &dst_slot.abs_cptr())
                    .unwrap();

                ib.caps_or_badges_mut()[0] = 0;
                sel4::reply(ib, rev_msg.extra_caps(1).build());

                dst_slot.delete().unwrap();
            }
            // 申请一个 Notification Capability
            RootEvent::AllocNotification => {
                // 在 0 的 slot 处创建一个 Capability
                OBJ_ALLOCATOR
                    .lock()
                    .retype_to_first(sel4::ObjectBlueprint::Notification);

                ib.caps_or_badges_mut()[0] = 0;
                sel4::reply(ib, rev_msg.extra_caps(1).build());

                LeafSlot::new(0).delete().unwrap();
            }
            RootEvent::Shutdown => sel4_kit::arch::shutdown(),
            RootEvent::Unknown(label) => {
                if label >= 8 {
                    log::error!("Unknown root messaage label: {label}")
                }
                let fault = with_ipc_buffer(|buffer| Fault::new(&buffer, &message));
                log::error!("[RootTask] Received Fault: {:?}", fault);
                sel4_kit::arch::shutdown();
                // match fault {
                //     _ => {}
                // }
            }
        }
    }
}
