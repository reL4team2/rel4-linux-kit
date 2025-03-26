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

/// PCNT 使用的中断号
pub const GENERIC_TIMER_PCNT_IRQ: usize = 30;

/// 获取当前的时间(ns)
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn get_curr_ns() -> usize {
    let cnt: usize;
    let freq: usize = get_freq();
    unsafe {
        core::arch::asm!("mrs  {}, cntpct_el0", out(reg) cnt);
    }
    cnt * 1_000_000_000 / freq
}

pub const US_PER_SEC: usize = 1_000_000;

/// 获取当前的时钟频率
#[inline]
fn get_freq() -> usize {
    let freq: usize;
    unsafe {
        core::arch::asm!("mrs  {}, cntfrq_el0", out(reg) freq);
    }
    freq
}

/// 获取当前的时间(us)
#[inline]
pub fn get_curr_us() -> usize {
    get_curr_ns() / 1000
}

/// 获取当前的时间(ms)
#[inline]
pub fn get_curr_ms() -> usize {
    get_curr_ns() / 1_000_000
}

/// 获取当前的时间 (sec)
#[inline]
pub fn get_curr_sec() -> usize {
    get_curr_ns() / 1_000_000_000
}

/// 设置定时器
#[inline]
pub fn set_timer(next_ns: usize) {
    let next_ticks = next_ns * get_freq() / 1_000_000_000;
    let enable = if next_ns != 0 { 1 } else { 0 };
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
pub fn get_cval_ns() -> usize {
    let cval: usize;
    unsafe {
        core::arch::asm!("mrs  {}, cntp_cval_el0", out(reg) cval);
    }
    cval * 1_000_000_000 / get_freq()
}
