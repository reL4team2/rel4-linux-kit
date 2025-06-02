//! 时钟相关的 crate
//!
//! 使用 `CNTP_xx_EL0` 来获取当前时间，进行定时器控制。
//! `cntp_ctl_el0`, `cntp_cval_el0`, `cntpct_el0`, `cntfrq_el0` 分别是控制寄存器、比较寄存器、计数寄存器、频率寄存器。
//!
//! `CNTP_CVAL_EL0` <https://developer.arm.com/documentation/ddi0601/2024-12/AArch64-Registers/CNTP-CVAL-EL0--Counter-timer-Physical-Timer-CompareValue-Register>
//! `CNTP_CTL_EL0` <https://developer.arm.com/documentation/ddi0601/2024-12/AArch64-Registers/CNTP-CTL-EL0--Counter-timer-Physical-Timer-Control-Register>
//! `CNTPCT_EL0` <https://developer.arm.com/documentation/ddi0601/2024-12/AArch64-Registers/CNTPCT-EL0--Counter-timer-Physical-Count-Register>
//! `CNTFRQ_EL0` <https://developer.arm.com/documentation/ddi0601/2024-12/AArch64-Registers/CNTFRQ-EL0--Counter-timer-Frequency-Register>
//!

use core::time::Duration;

/// PCNT 使用的中断号
pub const GENERIC_TIMER_PCNT_IRQ: usize = 30;

const NS_PER_SEC: usize = 1_000_000_000;

/// 获取当前的时间(ns)
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn current_time() -> core::time::Duration {
    let cnt: usize;
    let freq: usize = get_freq();
    unsafe {
        core::arch::asm!("mrs  {}, cntpct_el0", out(reg) cnt);
    }
    core::time::Duration::new((cnt / freq) as _, ((cnt % freq) * NS_PER_SEC / freq) as _)
}

/// 获取当前的时钟频率
#[inline]
fn get_freq() -> usize {
    let freq: usize;
    unsafe {
        core::arch::asm!("mrs  {}, cntfrq_el0", out(reg) freq);
    }
    freq
}

/// 设置定时器
#[inline]
pub fn set_timer(next: Duration) {
    let freq = get_freq();
    let next_ticks =
        next.as_secs() as usize * freq + next.subsec_nanos() as usize * freq / NS_PER_SEC;
    let enable = if next.is_zero() { 0 } else { 1 };
    unsafe {
        core::arch::asm!(
            "msr cntp_cval_el0, {}",
            "msr cntp_ctl_el0, {:x}",
            in(reg) next_ticks,
            in(reg) enable
        );
    }
}

/// 获取当前 timer 定时的时间
#[inline]
pub fn get_cval() -> Duration {
    let cval: usize;
    let freq = get_freq();
    unsafe {
        core::arch::asm!("mrs  {}, cntp_cval_el0", out(reg) cval);
    }
    core::time::Duration::new((cval / freq) as _, ((cval % freq) * NS_PER_SEC / freq) as _)
}
