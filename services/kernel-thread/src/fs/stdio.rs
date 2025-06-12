//! 标准输入输出使用的接口
//!
//! 目前标准输入输出等都使用一个结构体，通过设置不同的位置来确保只读，只写
use fs::INodeInterface;
use sel4::debug_print;
use syscalls::Errno;

use crate::device::uart::get_char;

/// 标准输入输出接口
pub struct StdConsole(u8);

impl StdConsole {
    /// 创建一个标准输入输出接口 [StdConsole]
    ///
    /// 根据不同的值，有不同的设置
    /// - `0` 只读
    /// - `1` | `2` 只写
    /// - `> 2` 可读可写
    pub const fn new(idx: u8) -> Self {
        Self(idx)
    }
}

impl INodeInterface for StdConsole {
    fn readat(&self, _offset: usize, buffer: &mut [u8]) -> vfscore::VfsResult<usize> {
        if self.0 != 0 && self.0 <= 2 {
            return Err(Errno::EPERM);
        }
        match get_char() {
            Some(c) => {
                buffer[0] = c;
                Ok(1)
            }
            None => Ok(0),
        }
    }

    fn writeat(&self, _offset: usize, buffer: &[u8]) -> vfscore::VfsResult<usize> {
        if self.0 == 0 {
            return Err(Errno::EPERM);
        }
        // srv_gate::UART_IMPLS[0].lock().puts(data);
        // debug_println!("{}", );
        buffer.iter().for_each(|x| debug_print!("{}", *x as char));
        Ok(buffer.len())
    }
}
