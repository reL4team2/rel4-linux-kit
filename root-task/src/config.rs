use config::{DMA_ADDR_START, PL011_ADDR, VIRTIO_MMIO_ADDR, VIRTIO_MMIO_VIRT_ADDR};

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

/// 任务列表，下列任务会在启动的时候被 root-task 加载到内存并分配指定的资源。
pub const TASK_FILES: &[KernelServices] = &[
    service! {
        name: "block-thread",
        file: "blk-thread.elf",
        mem: &[(VIRTIO_MMIO_VIRT_ADDR, VIRTIO_MMIO_ADDR, 0x1000)],
        dma: &[(DMA_ADDR_START, 0x2000)]
        // 可以添加 Shared 字段来提前设置预共享的页面
        // shared: &[(start, sharedid, usize)]
    },
    service! {
        name: "uart-thread",
        file: "uart-thread.elf",
        mem: &[(VIRTIO_MMIO_VIRT_ADDR, PL011_ADDR, 0x1000)],
    },
    // service! {
    //     name: "fs-thread",
    //     file: "ext4-thread.elf",
    // },
    service! {
        name: "kernel-thread",
        file: "kernel-thread.elf"
    },
    service! {
        name: "fs-thread",
        file: "lwext4-thread.elf",
    },
    // service! {
    //     name: "fs-thread",
    //     file: "fat-thread.elf",
    // },
    // service! {
    //     name: "simple-cli",
    //     file: "simple-cli.elf",
    // },
];
