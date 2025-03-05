//! 通过 IPC 关联文件系统
//!
//!

use alloc::{boxed::Box, string::String};
use common::services::{fs::FileSerivce, root::find_service};
use syscalls::Errno;

use super::vfs::{FileInterface, FileMetaData, FileResult, FileSystem};

/// 通过 IPC 连接的文件系统
pub struct IPCFileSystem {
    /// 文件系统名称，需要在编译期间指定名称
    pub name: &'static str,
    /// 传输端口
    pub fs: FileSerivce,
}

/// 通过 IPC 连接的文件系统
pub struct IPCFile {
    /// 文件路径
    pub path: String,
    /// 文件标识节点
    pub inode: u64,
    /// 文件大小
    pub fsize: u64,
    /// 传输端口
    pub fs: FileSerivce,
}

impl IPCFileSystem {
    /// 创建一个 IPC 文件系统
    ///
    /// - `name` 文件系统名称，也将使用这个名称查找服务的存在, 目前限制必须是编译期间存在的字符
    pub fn new(name: &'static str) -> Result<Self, sel4::Error> {
        Ok(Self {
            name,
            fs: find_service(name)?.into(),
        })
    }
}

impl FileSystem for IPCFileSystem {
    fn info(&self) -> super::vfs::FSInfo {
        todo!()
    }

    fn open(&self, path: &str, _flags: u64) -> FileResult<Box<dyn FileInterface>> {
        let (inode, fsize) = self.fs.open(path).unwrap();
        Ok(Box::new(IPCFile {
            path: String::from(path),
            inode: inode as _,
            fsize: fsize as _,
            fs: self.fs.clone(),
        }))
    }
}

impl FileInterface for IPCFile {
    fn readat(&mut self, off: usize, buf: &mut [u8]) -> FileResult<usize> {
        self.fs
            .read_at(self.inode, off, buf)
            .map_err(|_| Errno::EIO)
    }

    fn writeat(&mut self, _off: usize, _data: &[u8]) -> FileResult<usize> {
        todo!()
    }

    fn mkdir(&mut self, _name: &str) -> FileResult<()> {
        todo!()
    }

    fn metadata(&self) -> FileResult<super::vfs::FileMetaData> {
        Ok(FileMetaData {
            size: self.fsize as _,
        })
    }
}
