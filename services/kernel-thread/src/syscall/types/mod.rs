//! SysCall 使用到的类型定义
//!
//!
pub mod fs;
pub mod mm;
pub mod signal;
pub mod sys;
pub mod thread;
use zerocopy::{FromBytes, Immutable};

#[repr(C)]
#[derive(Clone, FromBytes, Immutable)]
pub(super) struct IoVec {
    pub base: usize,
    pub len: usize,
}
