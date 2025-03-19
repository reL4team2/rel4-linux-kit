use sel4::{UntypedDesc, cap::Untyped};

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
