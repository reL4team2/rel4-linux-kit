use crate_consts::GRANULE_SIZE;
use sel4::{cap::Untyped, init_thread, AbsoluteCPtr, HasCPtrWithDepth, UntypedDesc};
use sel4_root_task::debug_println;

use crate::FREE_PAGE_PLACEHOLDER;

#[repr(C, align(4096))]
pub struct FreePagePlaceHolder(#[allow(dead_code)] pub [u8; GRANULE_SIZE]);

/// Send a syscall to sel4 with none arguments
#[allow(dead_code)]
pub fn sys_null(sys: isize) {
    unsafe {
        core::arch::asm!(
            "svc 0",
            in("x7") sys,
        );
    }
}

/// unmap 空闲页，返回该页起始地址
pub unsafe fn init_free_page_addr(bootinfo: &sel4::BootInfo) -> usize {
    let addr = core::ptr::addr_of!(FREE_PAGE_PLACEHOLDER) as usize;
    get_user_image_frame_slot(bootinfo, addr)
        .cap()
        .frame_unmap()
        .unwrap();
    addr
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

/// Get [AbsoluteCPtr] from current CSpace though path.
pub fn abs_cptr<T: HasCPtrWithDepth>(path: T) -> AbsoluteCPtr {
    init_thread::slot::CNODE.cap().absolute_cptr(path)
}

/// Display the boot information in the console.
pub fn display_bootinfo(
    bootinfo: &sel4::BootInfoPtr,
    mem_untypes: &[(Untyped, &UntypedDesc)],
    dev_untypes: &[(Untyped, &UntypedDesc)],
) {
    debug_println!(
        "[RootTask] device untyped index range: {:?}",
        bootinfo.device_untyped_range()
    );
    debug_println!(
        "[RootTask] mem untyped index range: {:?}",
        bootinfo.kernel_untyped_range()
    );
    debug_println!(
        "[RootTask] untyped range: {:?}->{:?}",
        bootinfo.untyped().start(),
        bootinfo.untyped().end()
    );
    debug_println!(
        "[RootTask] empty slot range: {:?}",
        bootinfo.empty().range()
    );

    debug_println!("[RootTask] Untyped List: ");
    mem_untypes.iter().rev().for_each(|(cap, untyped)| {
        debug_println!(
            "    Untyped({:03}) paddr: {:#x?} size: {:#x}",
            cap.bits(),
            untyped.paddr(),
            (1usize << untyped.size_bits())
        );
    });
    dev_untypes.iter().rev().for_each(|(cap, untyped)| {
        debug_println!(
            "    Untyped({:03}) paddr: {:#x?} size: {:#x}",
            cap.bits(),
            untyped.paddr(),
            (1usize << untyped.size_bits())
        );
    });
}
