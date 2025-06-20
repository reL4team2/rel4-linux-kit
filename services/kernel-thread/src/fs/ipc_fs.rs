//! 通过 IPC 关联文件系统
//!
//!

use core::cmp;

use alloc::{boxed::Box, string::String};
use libc_core::types::Stat;
use srv_gate::FS_IMPLS;

use super::vfs::{FileInterface, FileMetaData, FileResult, FileSystem};

/// 通过 IPC 连接的文件系统
pub struct IPCFileSystem {
    /// 文件系统名称，需要在编译期间指定名称
    pub name: &'static str,
    /// 传输端口
    pub fs: usize,
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
    pub fs: usize,
}

impl IPCFileSystem {
    /// 创建一个 IPC 文件系统
    ///
    /// - `name` 文件系统名称，也将使用这个名称查找服务的存在, 目前限制必须是编译期间存在的字符
    pub fn new(name: &'static str, id: usize) -> Result<Self, sel4::Error> {
        Ok(Self { name, fs: id })
    }
}

impl FileSystem for IPCFileSystem {
    fn info(&self) -> super::vfs::FSInfo {
        todo!()
    }

    fn open(&self, path: &str, flags: u64) -> FileResult<Box<dyn FileInterface>> {
        // let (inode, fsize) = self.fs.open(path, flags)?;
        let (inode, fsize) = FS_IMPLS[self.fs].lock().open(path, flags as _)?;
        Ok(Box::new(IPCFile {
            path: String::from(path),
            inode: inode as _,
            fsize: fsize as _,
            fs: self.fs,
        }))
    }

    fn mkdir(&self, path: &str) -> FileResult<()> {
        // self.fs.mkdir(path).map_err(|_| Errno::EIO)
        FS_IMPLS[self.fs].lock().mkdir(path);
        Ok(())
    }

    fn unlink(&self, path: &str) -> FileResult<()> {
        // self.fs.unlink(path).map_err(|_| Errno::EBADF)
        FS_IMPLS[self.fs].lock().unlink(path);
        Ok(())
    }
}

impl FileInterface for IPCFile {
    fn readat(&mut self, off: usize, buf: &mut [u8]) -> FileResult<usize> {
        // self.fs
        //     .read_at(self.inode, off, buf)
        //     .map_err(|_| Errno::EIO)
        let buf_len = cmp::min(buf.len(), 0x4000);
        Ok(FS_IMPLS[self.fs]
            .lock()
            .read_at(self.inode, off, &mut buf[..buf_len]))
    }

    fn writeat(&mut self, off: usize, data: &[u8]) -> FileResult<usize> {
        // let rsize = self
        //     .fs
        //     .write_at(self.inode, off, data)
        //     .map_err(|_| Errno::EIO)?;
        let rsize = FS_IMPLS[self.fs].lock().write_at(self.inode, off, data);
        if off + rsize > self.fsize as _ {
            self.fsize = (off + rsize) as _;
        }
        Ok(rsize)
    }

    fn metadata(&self) -> FileResult<super::vfs::FileMetaData> {
        Ok(FileMetaData {
            size: self.fsize as _,
        })
    }

    fn getdents64(&self, offset: usize, buffer: &mut [u8]) -> FileResult<(usize, usize)> {
        // self.fs.getdents64(self.inode, offset, buffer)
        Ok(FS_IMPLS[self.fs]
            .lock()
            .getdents64(self.inode, offset, buffer))
    }

    fn stat(&self) -> FileResult<Stat> {
        // self.fs.stat(self.inode as _).map_err(|_| Errno::EBADFD)
        Ok(FS_IMPLS[self.fs].lock().stat(self.inode as _))
    }
}

impl Drop for IPCFile {
    fn drop(&mut self) {
        FS_IMPLS[self.fs].lock().close(self.inode as _);
    }
}
