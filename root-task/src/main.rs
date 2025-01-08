#![no_std]
#![no_main]
#![feature(never_type)]

extern crate alloc;

mod task;
mod thread;
mod utils;

use alloc::{string::String, vec::Vec};
use common::*;
use crate_consts::*;
use include_bytes_aligned::include_bytes_aligned;
use sel4::{
    cap::{LargePage, Untyped},
    cap_type::{Endpoint, Granule},
    debug_println,
    init_thread::slot,
    with_ipc_buffer, with_ipc_buffer_mut, Cap, MessageInfoBuilder, ObjectBlueprintArm, UntypedDesc,
};
use sel4_root_task::{root_task, Never};
use services::REG_LEN;
use spin::Mutex;
use task::*;
use utils::*;

/// Equivalent structure
/// pub struct KernelServices {
///     name: String,
///     file: &[u8]
///     mem: {
///         virtual_address: usize,
///         physical_address: usize,
///         map_size: usize,
///     },
///     irqs: []
/// }
///

static TASK_FILES: &[(&str, &[u8], &[(usize, usize, usize)])] = &[
    // (
    //     "kernel-thread",
    //     include_bytes_aligned!(16, "../../../build/kernel-thread.elf"),
    //     &[],
    // ),
    (
        "block-thread",
        include_bytes_aligned!(16, "../../build/blk-thread.elf"),
        // (Virtual address, physical address, mapping size).
        // If the physical address is equal to 0, a random regular memory will
        // be allocated. If an address is specified, the corresponding one will
        // be found from both regular memory and device memory. If it is
        // not found, panic !!
        &[(0x10_2000_0000, VIRTIO_MMIO_ADDR, 0x1000)],
    ),
    // (
    //     "fat-thread",
    //     include_bytes_aligned!(16, "../../../build/fat-thread.elf"),
    //     &[],
    // ),
    // (
    //     "net-thread",
    //     include_bytes_aligned!(16, "../../../build/net-thread.elf"),
    //     &[],
    // ),
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
    let fault_ep = OBJ_ALLOCATOR
        .lock()
        .allocate_and_retyped_fixed_sized::<Endpoint>();

    let mut tasks: Vec<Sel4Task> = Vec::new();

    for task in TASK_FILES.iter() {
        tasks.push(build_kernel_thread(
            (fault_ep, tasks.len() as _),
            task.0,
            task.1,
            unsafe { init_free_page_addr(bootinfo) },
        )?);
    }

    let (blk_device_untyped_cap, _) = device_untypes
        .iter()
        .find(|(_, desc)| {
            (desc.paddr()..(desc.paddr() + (1 << desc.size_bits()))).contains(&VIRTIO_MMIO_ADDR)
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

    tasks[0].map_large_page(VIRTIO_MMIO_VIRT_ADDR, blk_device_frame_cap);
    // Map DMA frame.
    let page = OBJ_ALLOCATOR
        .lock()
        .allocate_and_retyped_fixed_sized::<Granule>();
    tasks[0].map_page(DMA_ADDR_START, page);

    let page = OBJ_ALLOCATOR
        .lock()
        .allocate_and_retyped_fixed_sized::<Granule>();
    tasks[0].map_page(DMA_ADDR_START + PAGE_SIZE, page);

    // Start tasks
    run_tasks(&tasks);

    loop {
        handle_ep(tasks.as_mut_slice(), fault_ep)
    }
}

/// Handle the end point.
fn handle_ep(tasks: &mut [Sel4Task], fault_ep: Cap<Endpoint>) {
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
        services::root::RootMessageLabel::Ping => {
            with_ipc_buffer_mut(|ipc_buffer| sel4::reply(ipc_buffer, rev_msg.build()))
        }
        services::root::RootMessageLabel::TranslateAddr => {
            let addr = with_ipc_buffer(|ipc_buffer| ipc_buffer.msg_regs()[0]) as usize;

            let phys_addr = tasks[badge as usize]
                .mapped_page
                .get(&(addr & !0xfff))
                .map(|x| x.frame_get_address().unwrap())
                .unwrap();

            with_ipc_buffer_mut(|ipc_buffer| {
                ipc_buffer.msg_regs_mut()[0] = (phys_addr + addr % 0x1000) as _;
                sel4::reply(ipc_buffer, rev_msg.length(1).build())
            });
        }
        services::root::RootMessageLabel::FindService => {
            let task = with_ipc_buffer(|ipc_buffer| {
                let len = ipc_buffer.msg_regs()[0] as usize;
                let name = String::from_utf8_lossy(&ipc_buffer.msg_bytes()[REG_LEN..REG_LEN + len]);
                tasks.iter().find(|task| task.name == name)
            });
            match task {
                Some(task) => {
                    with_ipc_buffer_mut(|ipc_buffer| {
                        ipc_buffer.caps_or_badges_mut()[0] = task.srv_ep.bits();
                        sel4::reply(ipc_buffer, rev_msg.extra_caps(1).build())
                    });
                }
                None => {
                    // 发生错误时返回值 不为 -1
                    with_ipc_buffer_mut(|ipc_buffer| {
                        sel4::reply(ipc_buffer, rev_msg.label(1).build());
                    })
                }
            }
        }
        // Allocate a irq handler capability
        // Transfer it to the requested service
        services::root::RootMessageLabel::RegisterIRQ => {
            let irq = with_ipc_buffer(|ipc_buffer| ipc_buffer.msg_regs()[0]);

            let dst_slot = &slot::CNODE.cap().absolute_cptr(slot::NULL.cptr());
            slot::IRQ_CONTROL
                .cap()
                .irq_control_get(irq, dst_slot)
                .unwrap();

            with_ipc_buffer_mut(|buffer| {
                buffer.caps_or_badges_mut()[0] = 0;
                sel4::reply(buffer, rev_msg.extra_caps(1).build());
            });
            slot::CNODE
                .cap()
                .absolute_cptr(slot::NULL.cptr())
                .delete()
                .unwrap();
        }
        // Allocate a notification capability
        services::root::RootMessageLabel::AllocNotification => {
            OBJ_ALLOCATOR
                .lock()
                .retype_to_first(sel4::ObjectBlueprint::Notification);

            with_ipc_buffer_mut(|buffer| {
                buffer.caps_or_badges_mut()[0] = 0;
                sel4::reply(buffer, rev_msg.extra_caps(1).build());
            });

            slot::CNODE
                .cap()
                .absolute_cptr(slot::NULL.cptr())
                .delete()
                .unwrap();
        }
    }
}
