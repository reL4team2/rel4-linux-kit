//! 提供虚拟文件系统和文件相关接口
//!
//!

use alloc::{boxed::Box, string::String};
use common::services::fs::Stat;
use syscalls::Errno;
use zerocopy::{FromBytes, IntoBytes};

/// 文件相关操作的返回值
pub type FileResult<T> = Result<T, Errno>;

/// 文件系统信息结构
pub struct FSInfo {
    /// 文件系统名称
    pub name: String,
}

/// 文件元信息
#[derive(Debug, Clone, FromBytes, IntoBytes)]
pub struct FileMetaData {
    /// 文件大小
    pub size: usize,
}

/// 文件操作接口
pub trait FileInterface: Sync + Send {
    /// 在特定位置读取特定长度的值
    ///
    /// - `off` 读取的偏移，如果文件不支持从偏移读取，就会忽略这个位，比如 stdin
    /// - `buf` 读取数据后写入的缓冲区
    fn readat(&mut self, off: usize, buf: &mut [u8]) -> FileResult<usize>;
    /// 在特定位置读取特定长度的值
    ///
    /// - `off` 写入的偏移，如果文件不支持从偏移写入，就会忽略这个位，比如 stdout, stderr
    /// - `data` 需要写入的数据
    fn writeat(&mut self, off: usize, data: &[u8]) -> FileResult<usize>;

    /// 读取文件元数据信息
    fn metadata(&self) -> FileResult<FileMetaData>;

    /// 获取当前文件的状态信息
    fn stat(&self) -> FileResult<Stat>;

    /// 获取当前文件目录信息
    ///
    /// - `offset` 当前已经读取的文件数
    /// - `buffer` 读取后填入的缓冲区
    ///
    /// TIPS: 返回值的两个数，第一个是填入缓冲区的字符数，第二个是已经读取的文件数
    fn getdents64(&self, _offset: usize, _buffer: &mut [u8]) -> FileResult<(usize, usize)> {
        Err(Errno::ENOTDIR)
    }
}

/// 文件系统相关接口
pub trait FileSystem: Sync + Send {
    /// 读取文件系统信息
    fn info(&self) -> FSInfo;

    /// 打开文件
    ///
    /// - `path` 需要在当前文件系统打开的路径
    /// - `flags` 打开文件使用的 flags
    fn open(&self, path: &str, flags: u64) -> FileResult<Box<dyn FileInterface>>;

    /// 在指定的路径下创建一个文件夹
    ///
    /// - `path` 需要创建的文件夹名称
    fn mkdir(&self, path: &str) -> FileResult<()>;

    /// 删除一个指定的文件
    ///
    /// - `path` 需要删除的文件
    fn unlink(&self, path: &str) -> FileResult<()>;
}
