#![no_std]
#![no_main]
#![feature(never_type)]

extern crate alloc;

mod cspace;
mod utils;
mod vcpu;

use alloc::{sync::Arc, vec::Vec};
use common::{ObjectAllocator, slot::alloc_slot};
use include_bytes_aligned::include_bytes_aligned;
use sel4::{
    Fault, UntypedDesc, VCpuReg,
    cap::{IrqHandler, Untyped},
    cap_type, debug_println,
    init_thread::slot::{self, IRQ_CONTROL},
    sys::seL4_MessageInfo,
    with_ipc_buffer,
};
use sel4_root_task::{Never, root_task};
use vcpu::*;

/// Object 分配器，可以用来申请 Capability
pub(crate) static OBJ_ALLOCATOR: ObjectAllocator = ObjectAllocator::empty();

#[root_task(heap_size = 0x12_0000)]
fn main(bootinfo: &sel4::BootInfoPtr) -> sel4::Result<Never> {
    // 设置调试信息
    slot::TCB.cap().debug_name(b"root");
    // let log_level = match option_env!("LOG") {
    //     Some("error") => log::LevelFilter::Error,
    //     Some("warn") => log::LevelFilter::Warn,
    //     Some("info") => log::LevelFilter::Info,
    //     Some("debug") => log::LevelFilter::Debug,
    //     _ => log::LevelFilter::Debug,
    // };
    common::init_log!(log::LevelFilter::Debug);

    // 初始化 untyped object
    let dev_untyped_start = bootinfo.untyped().start();
    let mem_untyped_start = dev_untyped_start + bootinfo.kernel_untyped_range().start;
    let mut mem_untypes = bootinfo.untyped_list()[bootinfo.kernel_untyped_range()]
        .iter()
        .enumerate()
        .map(|(idx, ud)| {
            (
                Untyped::from_bits((mem_untyped_start + idx) as _),
                ud.clone(),
            )
        })
        .collect::<Vec<(Untyped, UntypedDesc)>>();
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
    common::slot::init(bootinfo.empty().range().start..0x1000);

    let len = mem_untypes.len();
    OBJ_ALLOCATOR.init(mem_untypes.remove(len - 4).0);

    // 重建 Capability 空间，构建为多级 CSpace
    cspace::rebuild_cspace();

    common::slot::init_slot_edge_handler(|slot| {
        OBJ_ALLOCATOR.extend_slot(slot);
    });

    // Used for fault and normal IPC ( Reuse )
    let fault_ep = OBJ_ALLOCATOR.alloc_endpoint();

    // 开始创建任务
    // static TEST_VM_FILE: &[u8] =
    //     include_bytes_aligned!(16, "../../examples/vmm-apps/build/test-entry.elf");
    static TEST_VM_FILE: &[u8] = include_bytes_aligned!(
        16,
        "../../../../monolithic/polyhal/target/aarch64-unknown-none-softfloat/release/example"
    );
    let task = build_kernel_thread(1, (fault_ep, 1), "test", TEST_VM_FILE)?;
    let notification = OBJ_ALLOCATOR.alloc_notification();
    sel4::init_thread::slot::TCB
        .cap()
        .tcb_bind_notification(notification)
        .unwrap();
    let irq_handler = alloc_slot();
    IRQ_CONTROL
        .cap()
        .irq_control_get(30, &irq_handler.abs_cptr())
        .unwrap();
    let irq_handler = irq_handler.cap::<cap_type::IrqHandler>();
    irq_handler
        .irq_handler_set_notification(notification)
        .unwrap();
    debug_println!("service {:#x?}", task.name);

    task.run();

    loop {
        let (message, tid) = fault_ep.recv(());
        // tid == 0 is timer interrupt
        if tid == 0 {
            debug_println!("receive cntp: {}", tid);
            task.tcb.tcb_suspend().unwrap();
            task.vcpu.vcpu_inject_irq(14, 15, 1, 0).unwrap();
            // task.vcpu.vcpu_ack_vppi(27).unwrap();
            let isr = task.vcpu.vcpu_read_regs(VCpuReg::ISR).unwrap();
            let regs = task.tcb.tcb_read_all_registers(true).unwrap();
            debug_println!("isr value: {:#x}", isr);
            irq_handler.irq_handler_ack().unwrap();
            task.vcpu
                .vcpu_write_regs(VCpuReg::ISR, isr | (1 << 10) | (1 << 7))
                .unwrap();
            task.tcb.tcb_resume().unwrap();
            continue;
        }
        let fault = with_ipc_buffer(|buffer| Fault::new(buffer, &message));
        match fault {
            Fault::VmFault(vm_fault) => {
                let esr = vm_fault.fsr();
                let addr = vm_fault.addr();
                let ec = (esr >> 26) & 0x3f;
                // Write size equals (sas + 1) * byte
                // let sas = (esr >> 22) & 0x3;
                let srt = (esr >> 16) & 0x1f;
                if ec != 0x24 || addr != 0x1000_0000 {
                    log::debug!("recv from {}, message: {:#x?}", tid, message);
                    log::debug!("label: {:#x?}", vm_fault);
                    log::error!("only can handle write to 0x1000_0000(uart device)");
                    break;
                }
                let mut regs = task.tcb.tcb_read_all_registers(true).unwrap();
                let reg_value = *regs.gpr(srt as _);
                *regs.pc_mut() = regs.pc() + 4;
                sel4::debug_put_char(reg_value as u8);
                task.tcb.tcb_write_registers(true, 1, &mut regs).unwrap();
            }
            _ => {
                log::debug!("recv from {}, message: {:#x?}", tid, message);
                log::debug!("label: {:#x?}", fault);
                log::error!("Unexpected Error");
                break;
            }
        }
    }
    sel4_kit::arch::shutdown();
}
