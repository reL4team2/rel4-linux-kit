//! 包含文件相关操作
//!
//! 可以直接使用，或者作为 FileDescriptor 使用

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use common::services::fs::Stat;
use syscalls::Errno;

use crate::syscall::SysResult;

use super::vfs::{FileInterface, FileResult};

/// 文件结构
pub struct File {
    /// 文件内部结构
    inner: Box<dyn FileInterface>,
    /// 文件路径
    path: String,
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
        let (fpath, ori_file) = super::get_mounted(path);
        let open_path = "/".to_string() + path.strip_prefix(&fpath).unwrap();
        Ok(Self {
            inner: ori_file.open(&open_path, flags)?,
            path: path.to_string(),
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
    pub fn mkdir(path: &str) -> SysResult {
        let (fpath, fs) = super::get_mounted(path);
        let open_path = "/".to_string() + path.strip_prefix(&fpath).unwrap();
        fs.mkdir(&open_path).map(|_| 0)
    }

    /// 获取文件路径
    #[inline]
    pub fn path(&self) -> String {
        self.path.clone()
    }

    /// 从 [FileInterface] 创建 [File]
    pub const fn from_raw(inner: Box<dyn FileInterface>) -> Self {
        Self {
            inner,
            off: 0,
            path: String::new(),
        }
    }

    /// 获取当前文件的状态信息
    #[inline]
    pub fn stat(&self) -> FileResult<Stat> {
        self.inner.stat()
    }

    /// 删除一个文件
    ///
    /// - `path` 需要删除的文件的路径
    #[inline]
    pub fn unlink(path: &str) -> FileResult<()> {
        let (fpath, ori_file) = super::get_mounted(path);
        let open_path = "/".to_string() + path.strip_prefix(&fpath).unwrap();
        ori_file.unlink(&open_path)
    }

    /// 设置偏移并返回设置后的值
    ///
    /// - `offset`  需要设置的偏移，偏移根据 `which` 的值有所不同
    /// - `which`
    ///   - `0` offset 为相对文件起始的位置
    ///   - `1` offset 为相对当前位置的
    ///   - `2` offset 为相对结束位置
    pub fn seek(&mut self, offset: isize, which: usize) -> usize {
        let new_off = match which {
            0 => offset as usize,
            1 => self.off + offset as usize,
            2 => self.inner.metadata().unwrap().size + offset as usize,
            _ => unreachable!(),
        };
        self.off = new_off;
        new_off
    }

    /// 读取文件的节点
    ///
    /// - `buffer` 读取文件后填入的缓冲区
    pub fn getdents64(&mut self, buffer: &mut [u8]) -> FileResult<usize> {
        let (rlen, offset) = self.inner.getdents64(self.off, buffer)?;
        self.off = offset;
        Ok(rlen)
    }
}

unsafe impl Send for File {}
