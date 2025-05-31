#![allow(unused)]

use core::fmt::Debug;

use config::*;

use crate::include_bytes_aligned;

/// 内核服务名称
pub struct KernelServices {
    /// 服务名称
    pub name: &'static str,
    /// 文件数据
    pub file: &'static [u8],
    /// 格式： (虚拟地址，物理地址，内存大小)
    pub mem: &'static [(usize, usize, usize)],
    /// 格式： (虚拟地址, 内存大小)
    pub dma: &'static [(usize, usize)],
}

impl Debug for KernelServices {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("KernelServices")
            .field("name", &self.name)
            .field("mem", &self.mem)
            .field("dma", &self.dma)
            .finish()
    }
}

/// 服务宏，帮助将服务引入到系统中来
macro_rules! service {
    (
        name: $name:expr,
        file: $file:expr,
        mem: &[$(($mem_virt:expr, $mem_phys:expr, $mem_size:expr)),*],
        dma: &[$(($dma_addr:expr, $dma_size:expr)),*]$(,)?
    ) => {
        KernelServices {
            name: $name,
            file: include_bytes_aligned!(16, concat!("../../target/", $file)),
            mem: &[$(($mem_virt, $mem_phys, $mem_size)),*],
            dma: &[$(($dma_addr, $dma_size)),*],
        }
    };
    (name: $name:expr,file: $file:expr $(,)?) => {
        service!(name: $name, file:$file, mem: &[], dma: &[])
    };
    (name: $name:expr,file: $file:expr, mem: &[$(($mem_virt:expr, $mem_phys:expr, $mem_size:expr)),*] $(,)?) => {
        service!(name: $name, file:$file, mem: &[$(($mem_virt, $mem_phys, $mem_size)),*], dma: &[])
    };
}

#[cfg(clippy)]
pub const TASK_FILES: &[KernelServices] = &[];

#[cfg(not(clippy))]
include!("autoconfig.rs");
