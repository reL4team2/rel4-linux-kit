use core::cmp;

use super::IpcBufferRW;
use num_enum::{FromPrimitive, IntoPrimitive};
use sel4::{MessageInfo, MessageInfoBuilder, cap::Endpoint, with_ipc_buffer, with_ipc_buffer_mut};
use slot_manager::LeafSlot;
use syscalls::Errno;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::consts::{IPC_DATA_LEN, REG_LEN};

#[derive(Debug, IntoPrimitive, FromPrimitive)]
#[repr(u64)]
pub enum FileEvent {
    Ping,
    Init,
    ReadDir,
    GetDents64,
    Open,
    ReadAt,
    WriteAt,
    Stat,
    Mkdir,
    Unlink,
    Close,
    #[num_enum(catch_all)]
    Unknown(u64),
}

bitflags::bitflags! {
    #[derive(Debug, Clone)]
    pub struct StatMode: u32 {
        const NULL  = 0;
        /// Type
        const TYPE_MASK = 0o170000;
        /// FIFO
        const FIFO  = 0o010000;
        /// character device
        const CHAR  = 0o020000;
        /// directory
        const DIR   = 0o040000;
        /// block device
        const BLOCK = 0o060000;
        /// ordinary regular file
        const FILE  = 0o100000;
        /// symbolic link
        const LINK  = 0o120000;
        /// socket
        const SOCKET = 0o140000;

        /// Set-user-ID on execution.
        const SET_UID = 0o4000;
        /// Set-group-ID on execution.
        const SET_GID = 0o2000;

        /// Read, write, execute/search by owner.
        const OWNER_MASK = 0o700;
        /// Read permission, owner.
        const OWNER_READ = 0o400;
        /// Write permission, owner.
        const OWNER_WRITE = 0o200;
        /// Execute/search permission, owner.
        const OWNER_EXEC = 0o100;

        /// Read, write, execute/search by group.
        const GROUP_MASK = 0o70;
        /// Read permission, group.
        const GROUP_READ = 0o40;
        /// Write permission, group.
        const GROUP_WRITE = 0o20;
        /// Execute/search permission, group.
        const GROUP_EXEC = 0o10;

        /// Read, write, execute/search by others.
        const OTHER_MASK = 0o7;
        /// Read permission, others.
        const OTHER_READ = 0o4;
        /// Write permission, others.
        const OTHER_WRITE = 0o2;
        /// Execute/search permission, others.
        const OTHER_EXEC = 0o1;
    }
}

#[repr(C)]
pub struct Dirent64 {
    pub ino: u64,      // 索引结点号
    pub off: i64,      // 到下一个dirent的偏移
    pub reclen: u16,   // 当前dirent的长度
    pub ftype: u8,     // 文件类型
    pub name: [u8; 0], // 文件名
}

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

#[derive(Clone, Debug)]
pub struct FileSerivce {
    ep_cap: Endpoint,
    share_addr: usize,
    share_size: usize,
}

impl FileSerivce {
    pub const fn from_bits(bits: u64) -> Self {
        Self::new(Endpoint::from_bits(bits))
    }

    pub const fn from_leaf_slot(ls: LeafSlot) -> Self {
        Self::from_bits(ls.raw() as _)
    }

    pub const fn leaf_slot(&self) -> LeafSlot {
        LeafSlot::new(self.ep_cap.bits() as _)
    }

    pub const fn new(endpoint: Endpoint) -> Self {
        Self {
            ep_cap: endpoint,
            share_addr: 0,
            share_size: 0,
        }
    }

    #[inline]
    pub fn call(&self, msg: MessageInfo) -> MessageInfo {
        self.ep_cap.call(msg)
    }

    pub fn ping(&self) -> Result<MessageInfo, ()> {
        let ping_msg = MessageInfoBuilder::default()
            .label(FileEvent::Ping.into())
            .build();
        Ok(self.call(ping_msg))
    }

    pub fn init(&mut self, channel_id: usize, addr: usize, size: usize) -> Result<MessageInfo, ()> {
        self.share_addr = addr;
        self.share_size = size;
        with_ipc_buffer_mut(|ib| ib.msg_regs_mut()[0] = channel_id as _);
        let ping_msg = MessageInfoBuilder::default()
            .label(FileEvent::Init.into())
            .length(1)
            .build();
        Ok(self.call(ping_msg))
    }

    #[inline]
    pub fn read_at(&self, inode: u64, offset: usize, buf: &mut [u8]) -> Result<usize, ()> {
        let trans_len = cmp::min(buf.len(), self.share_size);
        with_ipc_buffer_mut(|ib| {
            let regs = ib.msg_regs_mut();
            regs[0] = inode;
            regs[1] = offset as _;
            regs[2] = trans_len as u64;
        });
        let msg = MessageInfoBuilder::default()
            .label(FileEvent::ReadAt.into())
            .length(3)
            .build();
        let ret = self.call(msg);
        assert_eq!(ret.label(), 0);
        let rlen = with_ipc_buffer(|ib| ib.msg_regs()[0] as usize);
        let ptr = self.share_addr as *mut u8;
        unsafe {
            ptr.copy_to_nonoverlapping(buf.as_mut_ptr(), rlen);
        }
        Ok(rlen)
    }

    #[inline]
    pub fn write_at(&self, inode: u64, offset: usize, data: &[u8]) -> Result<usize, ()> {
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
            .label(FileEvent::WriteAt.into())
            .length(3 + data.len().div_ceil(REG_LEN))
            .build();
        let ret = self.call(msg);
        assert_eq!(ret.label(), 0);
        with_ipc_buffer(|ib| Ok(ib.msg_regs()[0] as usize))
    }

    pub fn open(&self, path: &str, flags: u64) -> Result<(usize, usize), Errno> {
        let mut len = 0;
        with_ipc_buffer_mut(|ib| {
            flags.write_buffer(ib, &mut len);
            path.write_buffer(ib, &mut len);
        });
        let msg = MessageInfoBuilder::default()
            .label(FileEvent::Open.into())
            .length(len)
            .build();
        let ret = self.call(msg);
        if ret.label() == 0 {
            with_ipc_buffer(|ib| Ok((ib.msg_regs()[0] as _, ib.msg_regs()[1] as _)))
        } else {
            Err(Errno::new(ret.label() as _))
        }
    }

    pub fn mkdir(&self, path: &str) -> Result<(), ()> {
        let mut len = 0;
        with_ipc_buffer_mut(|ib| path.write_buffer(ib, &mut len));
        let msg = MessageInfoBuilder::default()
            .label(FileEvent::Mkdir.into())
            .length(len)
            .build();
        let ret = self.call(msg);
        assert_eq!(ret.label(), 0);
        Ok(())
    }

    pub fn unlink(&self, path: &str) -> Result<(), ()> {
        let mut len = 0;
        with_ipc_buffer_mut(|ib| path.write_buffer(ib, &mut len));
        let msg = MessageInfoBuilder::default()
            .label(FileEvent::Unlink.into())
            .length(len)
            .build();
        let ret = self.call(msg);
        assert_eq!(ret.label(), 0);
        Ok(())
    }

    pub fn close(&self, inode: usize) -> Result<(), ()> {
        with_ipc_buffer_mut(|ib| inode.write_buffer(ib, &mut 0));
        let msg = MessageInfoBuilder::default()
            .label(FileEvent::Close.into())
            .length(1)
            .build();
        let ret = self.call(msg);
        assert_eq!(ret.label(), 0);
        Ok(())
    }

    pub fn stat(&self, inode: usize) -> Result<Stat, ()> {
        with_ipc_buffer_mut(|ib| inode.write_buffer(ib, &mut 0));
        let msg = MessageInfoBuilder::default()
            .label(FileEvent::Stat.into())
            .length(1)
            .build();
        let ret = self.call(msg);
        assert_eq!(ret.label(), 0);
        let stat = with_ipc_buffer(|ib| {
            let ptr = ib.msg_bytes().as_ptr() as *const Stat;
            unsafe { ptr.as_ref().unwrap().clone() }
        });
        Ok(stat)
    }

    pub fn getdents64(
        &self,
        inode: u64,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<(usize, usize), Errno> {
        with_ipc_buffer_mut(|ib| {
            inode.write_buffer(ib, &mut 0);
            offset.write_buffer(ib, &mut 1);
            buf.len().write_buffer(ib, &mut 2);
        });
        let msg = MessageInfoBuilder::default()
            .label(FileEvent::GetDents64.into())
            .length(3)
            .build();
        let ret = self.call(msg);
        assert_eq!(ret.label(), 0);
        with_ipc_buffer(|ib| {
            let rlen = ib.msg_regs()[0] as usize;
            let num = ib.msg_regs()[1] as usize;
            buf[..rlen].copy_from_slice(&ib.msg_bytes()[2 * REG_LEN..2 * REG_LEN + rlen]);
            Ok((rlen, num))
        })
    }
}

impl From<LeafSlot> for FileSerivce {
    fn from(value: LeafSlot) -> Self {
        Self::from_leaf_slot(value)
    }
}
