//! 设备文件系统
//!
//!
mod null;
mod stdio;
mod zero;

use alloc::{collections::btree_map::BTreeMap, string::String, sync::Arc, vec::Vec};
use fs::{FileType, INodeInterface};
use libc_core::types::{Stat, StatMode};
use syscalls::Errno;
use vfscore::{DirEntry, FileSystem, VfsResult};

use crate::fs::devfs::stdio::StdConsole;

/// 设备文件系统
pub struct DevFS {
    /// 根文件系统
    root_dir: Arc<DevDir>,
}

impl DevFS {
    /// 创建一个新的 [DevFS]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            root_dir: Arc::new(DevDir::new()),
        })
    }

    /// 根据一个 [DevDir] 创建一个 DevFS
    pub fn new_with_dir(dev: DevDir) -> Arc<Self> {
        Arc::new(Self {
            root_dir: Arc::new(dev),
        })
    }
}

impl FileSystem for DevFS {
    fn root_dir(&self) -> Arc<dyn INodeInterface> {
        Arc::new(DevDirContainer {
            inner: self.root_dir.clone(),
        })
    }

    fn name(&self) -> &str {
        "devfs"
    }
}

/// 设备文件夹
pub struct DevDir {
    map: BTreeMap<&'static str, Arc<dyn INodeInterface>>,
}

/// 设备文件夹容器
pub struct DevDirContainer {
    inner: Arc<DevDir>,
}

impl DevDir {
    /// 创建一个新的设备文件夹
    pub fn new() -> Self {
        let mut map: BTreeMap<&'static str, Arc<dyn INodeInterface>> = BTreeMap::new();
        map.insert("stdout", Arc::new(StdConsole::new(1)));
        map.insert("stderr", Arc::new(StdConsole::new(2)));
        map.insert("stdin", Arc::new(StdConsole::new(0)));
        map.insert("ttyv0", Arc::new(StdConsole::new(3)));
        map.insert("null", Arc::new(null::Null));
        map.insert("zero", Arc::new(zero::Zero));

        Self { map }
    }

    /// 添加一个新的文件
    pub fn add(&mut self, path: &'static str, node: Arc<dyn INodeInterface>) {
        self.map.insert(path, node);
    }
}

impl Default for DevDir {
    fn default() -> Self {
        Self::new()
    }
}

impl INodeInterface for DevDirContainer {
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        self.inner.map.get(name).cloned().ok_or(Errno::ENOENT)
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        Ok(self
            .inner
            .map
            .keys()
            .map(|name| DirEntry {
                filename: String::from(*name),
                len: 0,
                file_type: FileType::Device,
            })
            .collect())
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        stat.dev = 0;
        stat.ino = 1; // TODO: convert path to number(ino)
        stat.mode = StatMode::DIR; // TODO: add access mode
        stat.nlink = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = 0;
        stat.blksize = 512;
        stat.blocks = 0;
        stat.rdev = 0; // TODO: add device id
        Ok(())
    }
}
