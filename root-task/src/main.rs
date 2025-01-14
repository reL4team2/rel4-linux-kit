#![no_std]
#![no_main]
#![feature(never_type)]

extern crate alloc;

mod task;
mod thread;
mod utils;

use alloc::vec::Vec;
use common::services::IpcBufferRW;
use common::*;
use crate_consts::*;
use include_bytes_aligned::include_bytes_aligned;
use sel4::{
    cap::{LargePage, Untyped},
    cap_type::Endpoint,
    debug_println,
    init_thread::slot,
    with_ipc_buffer_mut, Cap, IpcBuffer, MessageInfoBuilder, ObjectBlueprintArm, UntypedDesc,
};
use sel4_root_task::{root_task, Never};
use slot_manager::LeafSlot;
use spin::Mutex;
use task::*;
use utils::*;

/// Equivalent structure
pub struct KernelServices {
    name: &'static str,
    file: &'static [u8],
    // (Virtual address, physical address, mapping size).
    // If the physical address is equal to 0, a random regular memory will
    // be allocated. If an address is specified, the corresponding one will
    // be found from both regular memory and device memory. If it is
    // not found, panic !!
    mem: &'static [(usize, usize, usize)],
    /// 格式： (开始地址, 内存大小)
    dma: &'static [(usize, usize)],
}

static TASK_FILES: &[KernelServices] = &[
    KernelServices {
        name: "block-thread",
        file: include_bytes_aligned!(16, "../../target/blk-thread.elf"),
        mem: &[(VIRTIO_MMIO_VIRT_ADDR, VIRTIO_MMIO_ADDR, 0x1000)],
        dma: &[(DMA_ADDR_START, 0x2000)],
    },
    KernelServices {
        name: "uart-thread",
        file: include_bytes_aligned!(16, "../../target/uart-thread.elf"),
        mem: &[(VIRTIO_MMIO_VIRT_ADDR, PL011_ADDR, 0x1000)],
        dma: &[],
    },
    KernelServices {
        name: "ext4-thread",
        file: include_bytes_aligned!(16, "../../target/ext4-thread.elf"),
        mem: &[],
        dma: &[],
    },
    // KernelServices {
    //     name: "simple-cli",
    //     file: include_bytes_aligned!(16, "../../target/simple-cli.elf"),
    //     mem: &[],
    //     dma: &[],
    // },
    KernelServices {
        name: "kernel-thread",
        file: include_bytes_aligned!(16, "../../target/kernel-thread.elf"),
        mem: &[],
        dma: &[],
    },
];

/// The object allocator for the root task.
pub(crate) static OBJ_ALLOCATOR: Mutex<ObjectAllocator> = Mutex::new(ObjectAllocator::empty());

/// free page placeholder
pub(crate) static mut FREE_PAGE_PLACEHOLDER: FreePagePlaceHolder =
    FreePagePlaceHolder([0; GRANULE_SIZE]);

#[root_task(heap_size = 0x12_0000)]
fn main(bootinfo: &sel4::BootInfoPtr) -> sel4::Result<Never> {
    // 设置调试信息
    slot::TCB.cap().debug_name(b"root");

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
    OBJ_ALLOCATOR
        .lock()
        .init(bootinfo.empty().range(), mem_untypes.pop().unwrap().0);

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
            unsafe { init_free_page_addr(bootinfo) },
        )?);
    }

    // 处理所有定义的任务
    TASK_FILES.iter().enumerate().for_each(|(t_idx, t)| {
        // 映射设备内存，应该保证映射到指定的物理地址上
        // 一般情况下映射的大小不会超过一个页
        // NOTICE: 目前不支持多个人物共享一个物理页表的情况
        // TODO: 实现申请指定物理内存到映射到特定物理地址的操作
        for (vaddr, paddr, size) in t.mem {
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
                    "RootTask Mapping {:#x} -> {:#x}",
                    start + i * PAGE_SIZE,
                    page.frame_get_address().unwrap()
                );
                tasks[t_idx].map_page(start + i * PAGE_SIZE, page);
            });
        }
    });

    run_tasks(&tasks);

    loop {
        with_ipc_buffer_mut(|ib| handle_ep(tasks.as_mut_slice(), fault_ep, ib))
    }
}

/// Handle the end point.
fn handle_ep(tasks: &mut [Sel4Task], fault_ep: Cap<Endpoint>, ib: &mut IpcBuffer) {
    let rev_msg = MessageInfoBuilder::default();
    let (message, badge) = fault_ep.recv(());
    let msg_label = match services::root::RootMessageLabel::try_from(message.label()) {
        Ok(label) => label,
        Err(_) => return,
    };
    debug_println!(
        "[RootTask] Recv <{:?}> len: {}",
        msg_label,
        message.length()
    );

    match msg_label {
        services::root::RootMessageLabel::Ping => sel4::reply(ib, rev_msg.build()),
        services::root::RootMessageLabel::TranslateAddr => {
            let mut off = 0;
            let addr = usize::read_buffer(ib, &mut off);

            let phys_addr = tasks[badge as usize]
                .mapped_page
                .get(&(addr & !0xfff))
                .map(|x| x.frame_get_address().unwrap())
                .unwrap();

            ib.msg_regs_mut()[0] = (phys_addr + addr % 0x1000) as _;
            sel4::reply(ib, rev_msg.length(off).build());
        }
        services::root::RootMessageLabel::FindService => {
            let name = <&str>::read_buffer(ib, &mut 0);
            let task = tasks.iter().find(|task| task.name == name);
            let msg = match task {
                Some(task) => {
                    ib.caps_or_badges_mut()[0] = task.srv_ep.bits();
                    rev_msg.extra_caps(1).build()
                }
                // 发生错误时返回值 不为 -1
                None => rev_msg.label(1).build(),
            };
            sel4::reply(ib, msg);
        }
        // Allocate a irq handler capability
        // Transfer it to the requested service
        services::root::RootMessageLabel::RegisterIRQ => {
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
        // Allocate a notification capability
        services::root::RootMessageLabel::AllocNotification => {
            // 在 0 的 slot 出创建一个 Capability
            OBJ_ALLOCATOR
                .lock()
                .retype_to_first(sel4::ObjectBlueprint::Notification);

            ib.caps_or_badges_mut()[0] = 0;
            sel4::reply(ib, rev_msg.extra_caps(1).build());

            LeafSlot::new(0).delete().unwrap();
        }
    }
}
