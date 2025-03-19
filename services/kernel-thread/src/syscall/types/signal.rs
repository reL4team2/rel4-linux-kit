//! 信号相关的定义
//!
//!

use num_enum::TryFromPrimitive;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// 信号屏蔽位处理
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(usize)]
pub enum SigMaskHow {
    /// 屏蔽一个信号
    Block = 0,
    /// 放开一个信号
    Unblock,
    /// 设置整个屏蔽位
    Setmask,
}

/// 信号屏蔽位
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, FromBytes, Immutable, KnownLayout, IntoBytes)]
pub struct SigProcMask(usize);

impl SigProcMask {
    /// 处理信号屏蔽位
    ///
    /// - `how`  处理方式
    ///   - [SigMaskHow::Block] 屏蔽一个信号
    ///   - [SigMaskHow::Unblock] 放开一个信号
    ///   - [SigMaskHow::Setmask] 将整个屏蔽位设置为 `mask`
    /// - `mask` 信号屏蔽位
    pub fn handle(&mut self, how: SigMaskHow, mask: &Self) {
        self.0 = match how {
            SigMaskHow::Block => self.0 | mask.0,
            SigMaskHow::Unblock => self.0 & (!mask.0),
            SigMaskHow::Setmask => mask.0,
        }
    }

    /// 检查信号是否被屏蔽
    ///
    /// - `signum` 信号编号
    pub fn masked(&self, signum: usize) -> bool {
        (self.0 >> signum) & 1 == 0
    }
}

/// 信号处理
#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, KnownLayout, Immutable, IntoBytes)]
pub struct SigAction {
    /// 信号处理程序
    pub handler: usize, // void     (*sa_handler)(int);
    /// 信号相关标志
    pub flags: usize, // int        sa_flags;
    /// 信号处理后返回的地址
    pub restorer: usize, // void     (*sa_restorer)(void);
    /// 执行信号过程中的屏蔽位
    pub mask: SigProcMask, // sigset_t   sa_mask;
}
