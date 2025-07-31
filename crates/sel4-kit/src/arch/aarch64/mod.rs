mod timer;

pub use timer::{GENERIC_TIMER_PCNT_IRQ, current_time, get_cval, set_timer};

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

///  回复一个消息
///
/// # 参数
///
/// - `sys` reply 使用的系统调用号
/// - `info` 回复的时候使用的 [sel4::sys::seL4_MessageInfo]
/// - `mrx` 回复使用的消息
#[cfg(not(feature = "mcs"))]
pub fn sys_reply(
    sys: isize,
    info: sel4::sys::seL4_MessageInfo,
    mr0: usize,
    mr1: usize,
    mr2: usize,
    mr3: usize,
) {
    unsafe {
        core::arch::asm!("svc 0",
            in("x7") sys,
            in("x1") info.0.into_inner()[0],
            in("x2") mr0,
            in("x3") mr1,
            in("x4") mr2,
            in("x5") mr3,
        );
    }
}
