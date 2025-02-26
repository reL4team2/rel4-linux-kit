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
