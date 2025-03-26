/// Arm Power State Coordination Interface
///               Platform Design Document
/// 手册: https://developer.arm.com/documentation/den0022/latest
/// 章节: CHAPTER 5.1.9 SYSTEM_OFF
const SYSMTEM_OFF: u32 = 0x8400_0008;

/// 关机指令
///
/// ```plain
/// psci {
///     migrate = <0xc4000005>;
///     cpu_on = <0xc4000003>;
///     cpu_off = <0x84000002>;
///     cpu_suspend = <0xc4000001>;
///     method = "smc";
///     compatible = "arm,psci-1.0", "arm,psci-0.2", "arm,psci";
/// };
/// ```
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
            x0: SYSMTEM_OFF as _,
            ..Default::default()
        })
        .unwrap();
    unreachable!()
}

/// 执行无参数的系统调用
///
/// - `sys` 需要执行的系统调用 id
pub fn sys_null(sys: isize) {
    unsafe {
        core::arch::asm!("svc 0",
            in("x7") sys,
        );
    }
}
