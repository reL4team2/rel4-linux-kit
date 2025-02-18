//! 每个任务都需要一个 IPCBuffer。
//! 对于宏内核的线程，因为是共享地址空间的，所以线程之间没办法在同一个地址
//! 共享 IPCBuffer，所以有两种解法:
//!
//! 第一种、多个线程共享一个 IPCBuffer，使用 IPCBuffer 的时候加锁，但
//! 是如果其中一个线程阻塞在内核，就会导致其他的线程无法进入内核
//!
//! 第二种、每个线程都单独拥有一个 IPCBuffer，每个程序都拥有一个特制的入
//! 口，在入口处分配 IPCBuffer，可能遇到的问题是分配和回收
use object::{File, Object};

use super::Sel4Task;

/// IPC Buffer 分配地址
const IPC_BUFFER_ADDR: usize = 0x10_0010_0000;
/// IPC Buffer 长度
const IPC_BUFFER_LEN: usize = 0x1000;

/// SHIM 程序
const SHIM_ELF: &[u8] = include_bytes!("../../../../target/shim.elf");

impl Sel4Task {
    /// 初始化 shim，加载 [SHIM_ELF] 到特定的内存，记录 SHIM 的入口地址
    fn init_shim(&mut self) {
        if self.info.shim_addr != 0 {
            return;
        }
        let file = File::parse(SHIM_ELF).unwrap();
        self.info.shim_addr = file.entry() as _;
        self.load_elf(SHIM_ELF);
    }
}
