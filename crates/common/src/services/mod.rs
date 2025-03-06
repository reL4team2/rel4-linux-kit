use alloc::string::{String, ToString};
use sel4::{IpcBuffer, MessageInfo, with_ipc_buffer_mut};

use crate::consts::REG_LEN;

pub mod block;
pub mod fs;
pub mod root;
pub mod uart;

// TODO: 设置一个新的 ipc_buffer 因为 ipc_buffer 是线程独占的，所以可以不加锁
// 使用 with_ipc_buffer 的形式很难受

macro_rules! impl_ipc_rw {
    ($($name:ident),*) => {
        $(
            impl IpcBufferRW for $name {
                type OutType = $name;
                #[inline]
                fn read_buffer(ib: &IpcBuffer, off: &mut usize) -> Self::OutType {
                    *off += 1;
                    ib.msg_regs()[*off - 1] as _
                }

                #[inline]
                fn write_buffer(self, ib: &mut IpcBuffer, off: &mut usize) {
                    ib.msg_regs_mut()[*off] = self as _;
                    *off += 1;
                }
            }
        )*
    };
}

// TODO: use &mut usize to record offset and set length in the future.
pub trait IpcBufferRW {
    type OutType;
    fn read_buffer(ib: &IpcBuffer, off: &mut usize) -> Self::OutType
    where
        Self: Sized;
    fn write_buffer(self, ib: &mut IpcBuffer, off: &mut usize);
}

impl_ipc_rw!(u8, u16, u32, u64, i8, i16, i32, i64, usize);

impl IpcBufferRW for &str {
    type OutType = String;
    #[inline]
    fn read_buffer(ib: &IpcBuffer, off: &mut usize) -> Self::OutType {
        let len = ib.msg_regs()[*off] as usize;
        let slice = &ib.msg_bytes()[(*off + 1) * REG_LEN..(*off + 1) * REG_LEN + len];
        let s = core::str::from_utf8(slice).unwrap();
        *off += 1 + len.div_ceil(REG_LEN);
        s.to_string()
    }

    #[inline]
    fn write_buffer(self, ib: &mut IpcBuffer, off: &mut usize) {
        let len = self.len();
        ib.msg_regs_mut()[*off] = len as _;
        ib.msg_bytes_mut()[(*off + 1) * REG_LEN..(*off + 1) * REG_LEN + len]
            .copy_from_slice(self.as_bytes());
        *off += 1 + len.div_ceil(REG_LEN);
    }
}

/// 回复一个 IPC 消息
#[inline]
pub fn sel4_reply(msg: MessageInfo) {
    with_ipc_buffer_mut(|ib| sel4::reply(ib, msg))
}
