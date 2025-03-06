/// 获取当前的时间(ns)
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn get_curr_ns() -> usize {
    let cnt: usize;
    let freq: usize;
    unsafe {
        core::arch::asm!("mrs  {}, cntpct_el0", out(reg) cnt);
        core::arch::asm!("mrs  {}, cntfrq_el0", out(reg) freq);
    }
    cnt * 1_000_000_000 / freq
}

pub const US_PER_SEC: usize = 1_000_000;

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
