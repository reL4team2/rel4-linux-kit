//! 系统调用的类型定义
//!
//!

use sel4_kit::arch::current_time;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// 系统信息
#[derive(Debug, FromBytes, Immutable, KnownLayout, IntoBytes)]
pub struct UtsName {
    /// 系统名称
    pub sysname: [u8; 65],
    /// 节点名称
    pub nodename: [u8; 65],
    /// 发布版本
    pub release: [u8; 65],
    /// 系统版本
    pub version: [u8; 65],
    /// 架构名称
    pub machine: [u8; 65],
    /// 域名
    pub domainname: [u8; 65],
}

/// 时间结构
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, FromBytes, Immutable, KnownLayout, IntoBytes)]
pub struct TimeVal {
    /// 秒
    pub sec: usize,
    /// 微秒，范围在 0~999999
    pub usec: usize,
}

impl TimeVal {
    /// 获取当前的时间
    pub fn now() -> Self {
        let time = current_time();
        Self {
            sec: time.as_secs() as _,
            usec: time.subsec_micros() as _,
        }
    }
}
