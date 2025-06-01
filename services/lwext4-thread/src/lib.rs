#![no_std]
#![feature(used_with_arg)]

extern crate alloc;

#[cfg(not(blk_ipc))]
extern crate blk_thread;

mod imp;

use core::iter::zip;

use alloc::string::String;
use common::services::{
    fs::{Dirent64, StatMode},
    root::create_channel,
};
use flatten_objects::FlattenObjects;
use imp::Ext4Disk;
use lwext4_rust::{
    Ext4BlockWrapper, Ext4File, InodeTypes,
    bindings::{O_CREAT, O_TRUNC},
};
use srv_gate::{
    BLK_IMPLS, def_fs_impl,
    fs::{FSIface, Stat},
};
use syscalls::Errno;

const O_DIRECTORY: u32 = 0o40000;
const STORE_CAP: usize = 500;

def_fs_impl!(EXT4FS, EXT4FSImpl::new());

pub struct EXT4FSImpl {
    _fs: Ext4BlockWrapper<Ext4Disk>,
    stores: FlattenObjects<Ext4File, STORE_CAP>,
}

unsafe impl Sync for EXT4FSImpl {}
unsafe impl Send for EXT4FSImpl {}

impl EXT4FSImpl {
    pub fn new() -> Self {
        let channel_id = create_channel(0x3_0000_0000, 4);
        BLK_IMPLS[0].lock().init(channel_id);
        EXT4FSImpl {
            _fs: Ext4BlockWrapper::new(Ext4Disk::new()).expect("Failed to create Ext4BlockWrapper"),
            stores: FlattenObjects::new(),
        }
    }
}

impl Default for EXT4FSImpl {
    fn default() -> Self {
        Self::new()
    }
}
impl FSIface for EXT4FSImpl {
    fn init(&mut self, _channel_id: usize, _addr: usize, _size: usize) {}

    fn read_at(&mut self, inode: u64, offset: usize, buf: &mut [u8]) -> usize {
        if let Some(ext4_file) = self.stores.get_mut(inode as _) {
            ext4_file.file_seek(offset as _, 0).unwrap();
            ext4_file.file_read(buf).unwrap()
        } else {
            panic!("Can't Find File")
        }
    }

    fn write_at(&mut self, inode: u64, offset: usize, data: &[u8]) -> usize {
        if let Some(ext4_file) = self.stores.get_mut(inode as _) {
            ext4_file.file_seek(offset as _, 0).unwrap();
            ext4_file.file_write(data).unwrap()
        } else {
            panic!("Can't Find File")
        }
    }

    fn open(&mut self, path: &str, flags: u32) -> Result<(usize, usize), Errno> {
        let mut ext4_file = Ext4File::new("/", lwext4_rust::InodeTypes::EXT4_DE_DIR);
        if flags & O_CREAT == O_CREAT {
            if flags & O_DIRECTORY != O_DIRECTORY {
                ext4_file = Ext4File::new(path, lwext4_rust::InodeTypes::EXT4_DE_REG_FILE);
                // FIXME: clean this O_TRUNC
                ext4_file.file_open(path, flags | O_TRUNC).unwrap();
            } else {
                panic!("Just support create regular file");
            }
        } else if ext4_file.check_inode_exist(path, InodeTypes::EXT4_DE_DIR) {
            ext4_file = Ext4File::new(path, lwext4_rust::InodeTypes::EXT4_DE_DIR);
        } else if ext4_file.check_inode_exist(path, InodeTypes::EXT4_DE_REG_FILE) {
            ext4_file = Ext4File::new(path, lwext4_rust::InodeTypes::EXT4_DE_REG_FILE);
            ext4_file.file_open(path, flags).unwrap();
        } else {
            // sel4::reply(ib, rev_msg.label(Errno::EACCES.into_raw() as _).build());
            return Err(Errno::EACCES);
        }

        let file_size = ext4_file.file_size();
        if let Ok(index) = self.stores.add(ext4_file) {
            Ok((index as _, file_size as _))
        } else {
            panic!("Can't add files");
        }
    }

    fn mkdir(&self, path: &str) {
        let mut ext4_file = Ext4File::new(path, lwext4_rust::InodeTypes::EXT4_DE_DIR);
        ext4_file.dir_mk(path).unwrap();
    }

    fn unlink(&self, path: &str) {
        let mut ext4_file = Ext4File::new(path, lwext4_rust::InodeTypes::EXT4_DE_DIR);
        ext4_file.file_remove(path).unwrap();
    }

    fn close(&mut self, inode: usize) {
        if let Some(mut ext4_file) = self.stores.remove(inode) {
            ext4_file.file_close().unwrap();
        }
    }

    fn stat(&mut self, inode: usize) -> Stat {
        if let Some(ext4_file) = self.stores.get_mut(inode) {
            let mode = ext4_file.file_mode_get().unwrap()
                | match ext4_file.get_type() {
                    InodeTypes::EXT4_DE_REG_FILE => StatMode::FILE,
                    InodeTypes::EXT4_DE_DIR => StatMode::DIR,
                    InodeTypes::EXT4_DE_CHRDEV => StatMode::CHAR,
                    InodeTypes::EXT4_DE_BLKDEV => StatMode::BLOCK,
                    InodeTypes::EXT4_DE_FIFO => StatMode::FIFO,
                    InodeTypes::EXT4_DE_SOCK => StatMode::SOCKET,
                    InodeTypes::EXT4_DE_SYMLINK => StatMode::LINK,
                    _ => StatMode::FILE,
                }
                .bits();
            Stat {
                blksize: 0x200,
                ino: inode as _,
                mode,
                nlink: 1,
                size: ext4_file.file_size(),
                ..Default::default()
            }
        } else {
            panic!("Can't Find File")
        }
    }

    fn getdents64(&mut self, inode: u64, mut offset: usize, buf: &mut [u8]) -> (usize, usize) {
        if let Some(ext4_file) = self.stores.get_mut(inode as _) {
            let entries = ext4_file.lwext4_dir_entries().unwrap();
            let mut real_rlen: usize = 0;
            let mut base_ptr = buf.as_ptr() as usize;
            for (name, ty) in zip(entries.0, entries.1).skip(offset) {
                log::debug!("{:?} , {:?}", String::from_utf8(name.clone()), ty);
                let len = name.len() + size_of::<Dirent64>();
                let aligned = (len + 7) / 8 * 8;
                if real_rlen + aligned > buf.len() {
                    break;
                }
                let dirent = unsafe { (base_ptr as *mut Dirent64).as_mut() }.unwrap();
                dirent.ftype = 0;
                dirent.reclen = aligned as _;
                dirent.ino = 0;
                dirent.off = (real_rlen + aligned) as _;
                unsafe {
                    dirent
                        .name
                        .as_mut_ptr()
                        .copy_from(name.as_ptr(), name.len());
                }
                real_rlen += aligned;
                base_ptr += aligned;
                offset += 1;
            }
            (real_rlen as _, offset as _)
        } else {
            panic!("Can't find folder")
        }
    }
}
