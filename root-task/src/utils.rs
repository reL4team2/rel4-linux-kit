use crate_consts::GRANULE_SIZE;
use sel4::{cap::Untyped, UntypedDesc};

use crate::FREE_PAGE_PLACEHOLDER;

#[repr(C, align(4096))]
pub struct FreePagePlaceHolder(#[allow(dead_code)] pub [u8; GRANULE_SIZE]);

/// unmap 空闲页，返回该页起始地址
pub unsafe fn init_free_page_addr(bootinfo: &sel4::BootInfo) -> usize {
    let addr = core::ptr::addr_of!(FREE_PAGE_PLACEHOLDER) as usize;
    get_user_image_frame_slot(bootinfo, addr)
        .cap()
        .frame_unmap()
        .unwrap();
    addr
}

/// 关机指令
#[allow(dead_code)]
pub fn shutdown() -> ! {
    // use sel4::InvocationContext;
    // init_thread::slot::CNODE.cap().into_invocation_context().with_context(|ipc_buffer| {
    //     let mut resp = sel4::sys::seL4_ARM_SMCContext::default();
    //     ipc_buffer.inner_mut().seL4_ARM_SMC_Call(
    //         sel4::sys::seL4_RootCNodeCapSlots::seL4_CapSMC as _,
    //         &sel4::sys::seL4_ARM_SMCContext {
    //             x0: 0x8400_0008,
    //             ..Default::default()
    //         },
    //         &mut resp
    //     );
    // });
    sel4::init_thread::slot::SMC
        .cap()
        .smc_call(&sel4::sys::seL4_ARM_SMCContext {
            x0: 0x8400_0008,
            ..Default::default()
        })
        .unwrap();
    unreachable!()
}

fn get_user_image_frame_slot(
    bootinfo: &sel4::BootInfo,
    addr: usize,
) -> sel4::init_thread::Slot<sel4::cap_type::Granule> {
    extern "C" {
        static __executable_start: usize;
    }
    let user_image_addr = core::ptr::addr_of!(__executable_start) as usize;
    bootinfo
        .user_image_frames()
        .index(addr / GRANULE_SIZE - user_image_addr / GRANULE_SIZE)
}

/// Display the boot information in the console.
pub fn display_bootinfo(
    bootinfo: &sel4::BootInfoPtr,
    mem_untypes: &[(Untyped, &UntypedDesc)],
    dev_untypes: &[(Untyped, &UntypedDesc)],
) {
    log::info!(
        "[RootTask] device untyped index range: {:?}",
        bootinfo.device_untyped_range()
    );
    log::info!(
        "[RootTask] mem untyped index range: {:?}",
        bootinfo.kernel_untyped_range()
    );
    log::info!(
        "[RootTask] untyped range: {:?}->{:?}",
        bootinfo.untyped().start(),
        bootinfo.untyped().end()
    );
    log::info!(
        "[RootTask] empty slot range: {:?}",
        bootinfo.empty().range()
    );

    log::info!("[RootTask] Untyped List: ");
    mem_untypes.iter().rev().for_each(|(cap, untyped)| {
        log::info!(
            "    Untyped({:03}) paddr: {:#x?} size: {:#x}",
            cap.bits(),
            untyped.paddr(),
            (1usize << untyped.size_bits())
        );
    });
    dev_untypes.iter().rev().for_each(|(cap, untyped)| {
        log::info!(
            "    Untyped({:03}) paddr: {:#x?} size: {:#x}",
            cap.bits(),
            untyped.paddr(),
            (1usize << untyped.size_bits())
        );
    });
}
