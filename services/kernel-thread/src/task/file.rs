use alloc::{string::String, sync::Arc};
use flatten_objects::FlattenObjects;
use fs::{file::File, pathbuf::PathBuf};
use libc_core::fcntl::{AT_FDCWD, OpenFlags};
use spin::Mutex;
use syscalls::Errno;
use vfscore::VfsResult;

use super::Sel4Task;

#[derive(Clone)]
pub struct TaskFileInfo {
    /// 工作目录
    pub work_dir: File,
    /// 文件描述符
    pub file_ds: Arc<Mutex<FlattenObjects<Arc<File>, 0x200>>>,
}

impl Default for TaskFileInfo {
    fn default() -> Self {
        let file_ds = Arc::new(Mutex::new(FlattenObjects::new()));
        Self {
            work_dir: File::open("/", OpenFlags::DIRECTORY).unwrap(),
            file_ds,
        }
    }
}

impl Sel4Task {
    /// 根据 dir_fd 打开一个文件
    ///
    /// ## 参数
    /// - `fd` 打开文件所在的文件夹
    /// - `path` 需要打开的文件的路径
    /// - `flags` 打开文件使用的标志
    pub fn fd_open(&self, dirfd: isize, path: *const u8, flags: OpenFlags) -> VfsResult<File> {
        let path = self.fd_resolve(dirfd, path)?;
        File::open(path, flags)
    }

    /// 根据 dir_fd 解析一个文件的真实路径
    ///
    /// ## 参数
    /// - `fd` 文件所在的文件夹
    /// - `path` 文件路径
    pub fn fd_resolve(&self, dirfd: isize, path: *const u8) -> VfsResult<PathBuf> {
        let path_bytes = self.read_cstr(path as _).unwrap();
        let filename = String::from_utf8(path_bytes).unwrap();

        if filename.starts_with("/") {
            Ok(filename.into())
        } else {
            let parent = match dirfd {
                AT_FDCWD => self.file.work_dir.clone(),
                _ => self
                    .file
                    .file_ds
                    .lock()
                    .get(dirfd as _)
                    .ok_or(Errno::EBADF)?
                    .as_ref()
                    .clone(),
            };
            Ok(parent.path_buf().join(&filename))
        }
    }
}
