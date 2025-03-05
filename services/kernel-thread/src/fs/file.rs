//! 包含文件相关操作
//!
//! 可以直接使用，或者作为 FileDescriptor 使用

use alloc::{boxed::Box, vec::Vec};
use syscalls::Errno;

use crate::syscall::SysResult;

use super::vfs::FileInterface;

/// 文件结构
pub struct File {
    /// 文件内部结构
    inner: Box<dyn FileInterface>,
    /// 文件读取或写入的偏移
    off: usize,
}

impl File {
    /// 打开文件
    ///
    /// - `path` 需要打开的文件的路径
    /// - `flags` 打开文件的时候使用的标志位
    pub fn open(path: &str, flags: u64) -> Result<Self, Errno> {
        // FIXME: 打开的时候丢弃 mount 的路径
        let (_, ori_file) = super::get_mounted(path);
        Ok(Self {
            inner: ori_file.open(path, flags)?,
            off: 0,
        })
    }

    /// 读取文件
    ///
    /// - `buffer` 读取数据后写入的缓冲区
    #[inline]
    pub fn read(&mut self, buffer: &mut [u8]) -> SysResult {
        self.inner
            .readat(self.off, buffer)
            .inspect(|rlen| self.off += rlen)
    }

    /// 读取整个文件
    ///
    /// 读取整个文件并存储在 [Vec<u8>] 中返回
    #[inline]
    pub fn read_all(&mut self) -> Result<Vec<u8>, Errno> {
        let last_size = self.inner.metadata()?.size - self.off;
        let mut buffer = vec![0u8; last_size];
        let mut readed_len = 0;
        loop {
            let rlen = self.inner.readat(self.off, &mut buffer[readed_len..])?;
            if rlen == 0 {
                break;
            }
            readed_len += rlen;
            self.off += rlen;
        }
        Ok(buffer)
    }

    /// 写入文件
    ///
    /// - `data` 需要写入的数据
    #[inline]
    pub fn write(&mut self, data: &[u8]) -> SysResult {
        self.inner
            .writeat(self.off, data)
            .inspect(|wlen| self.off += wlen)
    }

    /// 创建文件夹
    ///
    /// - `name` 创建文件夹使用的名称
    #[inline]
    pub fn mkdir(&mut self, name: &str) -> SysResult {
        self.inner.mkdir(name).map(|_| 0)
    }
}
