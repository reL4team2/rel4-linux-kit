use crate::__prelude::*;
use common::ipc_trait;
use sel4::MessageInfo;
use syscalls::Errno;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, IntoBytes, Immutable, FromBytes, KnownLayout)]
pub struct TimeSpec {
    pub sec: usize,  // 秒
    pub nsec: usize, // 纳秒, 范围在0~999999999
}

#[repr(C)]
#[derive(Debug, Default, Clone, IntoBytes, Immutable)]
#[cfg(not(target_arch = "x86_64"))]
pub struct Stat {
    pub dev: u64,        // 设备号
    pub ino: u64,        // inode
    pub mode: u32,       // 设备mode
    pub nlink: u32,      // 文件links
    pub uid: u32,        // 文件uid
    pub gid: u32,        // 文件gid
    pub rdev: u64,       // 文件rdev
    pub __pad: u64,      // 保留
    pub size: u64,       // 文件大小
    pub blksize: u32,    // 占用块大小
    pub __pad2: u32,     // 保留
    pub blocks: u64,     // 占用块数量
    pub atime: TimeSpec, // 最后访问时间
    pub mtime: TimeSpec, // 最后修改时间
    pub ctime: TimeSpec, // 最后创建时间
}

#[ipc_trait(event = FS_EVENT)]
pub trait FSIface: Sync + Send {
    fn init(&mut self, channel_id: usize, addr: usize, size: usize);
    fn read_at(&mut self, inode: u64, offset: usize, buf: &mut [u8]) -> usize;
    fn write_at(&mut self, inode: u64, offset: usize, data: &[u8]) -> usize;
    fn open(&mut self, path: &str, flags: u32) -> Result<(usize, usize), Errno>;
    fn mkdir(&self, path: &str);
    fn unlink(&self, path: &str);
    fn close(&mut self, inode: usize);
    fn stat(&mut self, inode: usize) -> Stat;
    fn getdents64(&mut self, inode: u64, offset: usize, buf: &mut [u8]) -> (usize, usize);
}

#[cfg(fs_ipc)]
mod _impl {
    use core::cmp;

    use super::{FSIface, FSIfaceEvent, Stat};
    use crate::def_fs_impl;
    use common::{
        consts::{IPC_DATA_LEN, REG_LEN},
        generate_ipc_send,
        services::root::find_service,
    };
    use sel4::{MessageInfoBuilder, cap::Endpoint, with_ipc_buffer, with_ipc_buffer_mut};
    use syscalls::Errno;

    def_fs_impl!(FS_IPC, FSIfaceIPCImpl {
        ep: find_service("fs-thread").unwrap().into(),
        share_addr: 0,
        share_size: 0
    });

    #[derive(Clone, Copy, Debug)]
    pub struct FSIfaceIPCImpl {
        ep: Endpoint,
        share_addr: usize,
        share_size: usize,
    }

    impl FSIface for FSIfaceIPCImpl {
        fn init(&mut self, channel_id: usize, addr: usize, size: usize) {
            self.share_addr = addr;
            self.share_size = size;
            with_ipc_buffer_mut(|ib| ib.msg_regs_mut()[0] = channel_id as _);
            let ping_msg = MessageInfoBuilder::default()
                .label(FSIfaceEvent::init.into())
                .length(1)
                .build();
            self.ep.call(ping_msg);
        }

        #[inline]
        fn read_at(&mut self, inode: u64, offset: usize, buf: &mut [u8]) -> usize {
            let trans_len = cmp::min(buf.len(), self.share_size);
            with_ipc_buffer_mut(|ib| {
                let regs = ib.msg_regs_mut();
                regs[0] = inode;
                regs[1] = offset as _;
                regs[2] = trans_len as u64;
            });
            let msg = MessageInfoBuilder::default()
                .label(FSIfaceEvent::read_at.into())
                .length(3)
                .build();
            let ret = self.ep.call(msg);
            assert_eq!(ret.label(), 0);
            let rlen = with_ipc_buffer(|ib| ib.msg_regs()[0] as usize);
            let ptr = self.share_addr as *mut u8;
            unsafe {
                ptr.copy_to_nonoverlapping(buf.as_mut_ptr(), rlen);
            }
            rlen
        }

        #[inline]
        fn write_at(&mut self, inode: u64, offset: usize, data: &[u8]) -> usize {
            let trans_len = cmp::min(data.len(), IPC_DATA_LEN - 3 * REG_LEN);
            with_ipc_buffer_mut(|ib| {
                let regs = ib.msg_regs_mut();
                regs[0] = inode;
                regs[1] = offset as _;
                regs[2] = data.len() as _;
                ib.msg_bytes_mut()[3 * REG_LEN..3 * REG_LEN + trans_len]
                    .copy_from_slice(&data[..trans_len]);
            });
            let msg = MessageInfoBuilder::default()
                .label(FSIfaceEvent::write_at.into())
                .length(3 + data.len().div_ceil(REG_LEN))
                .build();
            let ret = self.ep.call(msg);
            assert_eq!(ret.label(), 0);
            with_ipc_buffer(|ib| ib.msg_regs()[0] as usize)
        }

        fn open(&mut self, path: &str, flags: u32) -> Result<(usize, usize), Errno> {
            let mut len = 0;
            with_ipc_buffer_mut(|ib| {
                ib.msg_regs_mut()[0] = flags as _;
                ib.msg_regs_mut()[1] = path.len() as _;
                len += 2;
                ib.msg_bytes_mut()[REG_LEN * len..][..path.len()].copy_from_slice(path.as_bytes());
                len += path.len().div_ceil(REG_LEN);
            });
            let msg = MessageInfoBuilder::default()
                .label(FSIfaceEvent::open.into())
                .length(len)
                .build();
            let ret = self.ep.call(msg);
            if ret.label() == 0 {
                with_ipc_buffer(|ib| Ok((ib.msg_regs()[0] as _, ib.msg_regs()[1] as _)))
            } else {
                Err(Errno::new(ret.label() as _))
            }
        }

        #[generate_ipc_send(label = FSIfaceEvent::mkdir)]
        fn mkdir(&self, path: &str) {}

        #[generate_ipc_send(label = FSIfaceEvent::unlink)]
        fn unlink(&self, path: &str) {}

        #[generate_ipc_send(label = FSIfaceEvent::close)]
        fn close(&mut self, inode: usize) {}

        fn stat(&mut self, inode: usize) -> Stat {
            with_ipc_buffer_mut(|ib| ib.msg_regs_mut()[0] = inode as _);
            let msg = MessageInfoBuilder::default()
                .label(FSIfaceEvent::stat.into())
                .length(1)
                .build();
            let ret = self.ep.call(msg);
            assert_eq!(ret.label(), 0);
            let stat = with_ipc_buffer(|ib| {
                let ptr = ib.msg_bytes().as_ptr() as *const Stat;
                unsafe { ptr.as_ref().unwrap().clone() }
            });
            stat
        }

        fn getdents64(&mut self, inode: u64, offset: usize, buf: &mut [u8]) -> (usize, usize) {
            with_ipc_buffer_mut(|ib| {
                ib.msg_regs_mut()[0] = inode;
                ib.msg_regs_mut()[1] = offset as _;
                ib.msg_regs_mut()[2] = buf.len() as _;
            });
            let msg = MessageInfoBuilder::default()
                .label(FSIfaceEvent::getdents64.into())
                .length(3)
                .build();
            let ret = self.ep.call(msg);
            assert_eq!(ret.label(), 0);
            with_ipc_buffer(|ib| {
                let rlen = ib.msg_regs()[0] as usize;
                let num = ib.msg_regs()[1] as usize;
                buf[..rlen].copy_from_slice(&ib.msg_bytes()[2 * REG_LEN..2 * REG_LEN + rlen]);
                (rlen, num)
            })
        }
    }
}
