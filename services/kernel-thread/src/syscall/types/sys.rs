//! 系统调用的类型定义
//!
//!

use common::arch::{US_PER_SEC, get_curr_us};
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
        let us = get_curr_us();
        Self {
            sec: us / US_PER_SEC,
            usec: us % US_PER_SEC,
        }
    }
}
