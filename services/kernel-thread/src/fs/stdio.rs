//! 标准输入输出使用的接口
//!
//! 目前标准输入输出等都使用一个结构体，通过设置不同的位置来确保只读，只写

use syscalls::Errno;

use crate::device::uart::get_char;

use super::vfs::FileInterface;

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

impl FileInterface for StdConsole {
    fn readat(&mut self, _off: usize, buf: &mut [u8]) -> super::vfs::FileResult<usize> {
        if self.0 != 0 && self.0 <= 2 {
            return Err(Errno::EPERM);
        }
        match get_char() {
            Some(c) => {
                buf[0] = c;
                Ok(1)
            }
            None => Ok(0),
        }
    }

    fn writeat(&mut self, _off: usize, data: &[u8]) -> super::vfs::FileResult<usize> {
        if self.0 == 0 {
            return Err(Errno::EPERM);
        }
        data.iter().for_each(|c| sel4::debug_put_char(*c));
        Ok(data.len())
    }

    fn metadata(&self) -> super::vfs::FileResult<super::vfs::FileMetaData> {
        todo!()
    }

    fn stat(&self) -> super::vfs::FileResult<common::services::fs::Stat> {
        todo!()
    }
}
