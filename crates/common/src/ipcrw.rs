#[cfg(feature = "alloc")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use sel4::IpcBuffer;

use crate::config::REG_LEN;

macro_rules! impl_ipc_rw {
    ($($name:ident),*) => {
        $(
            impl IpcTypeReader for $name {
                type OutType = $name;
                #[inline]
                fn read_buffer(ib: &IpcBuffer, off: &mut usize) -> Self::OutType {
                    *off += 1;
                    ib.msg_regs()[*off - 1] as _
                }
            }

            impl IpcTypeWriter for $name {
                #[inline]
                fn write_buffer(self, ib: &mut IpcBuffer, off: &mut usize) {
                    ib.msg_regs_mut()[*off] = self as _;
                    *off += 1;
                }
            }
        )*
    };
}

pub trait IpcTypeReader {
    type OutType;
    fn read_buffer(ib: &IpcBuffer, off: &mut usize) -> Self::OutType;
}

pub trait IpcTypeWriter {
    fn write_buffer(self, ib: &mut IpcBuffer, off: &mut usize);
}

impl_ipc_rw!(u8, u16, u32, u64, i8, i16, i32, i64, usize);

#[cfg(feature = "alloc")]
impl IpcTypeReader for &str {
    type OutType = String;

    #[inline]
    fn read_buffer(ib: &IpcBuffer, off: &mut usize) -> Self::OutType {
        let len = ib.msg_regs()[*off] as usize;
        let slice = &ib.msg_bytes()[(*off + 1) * REG_LEN..][..len];
        *off += 1 + len.div_ceil(REG_LEN);
        String::from_utf8_lossy(slice).to_string()
    }
}

#[cfg(feature = "alloc")]
impl IpcTypeReader for String {
    type OutType = String;

    #[inline]
    fn read_buffer(ib: &IpcBuffer, off: &mut usize) -> Self::OutType {
        let len = ib.msg_regs()[*off] as usize;
        let slice = &ib.msg_bytes()[(*off + 1) * REG_LEN..][..len];
        *off += 1 + len.div_ceil(REG_LEN);
        String::from_utf8_lossy(slice).to_string()
    }
}

#[cfg(feature = "alloc")]
impl IpcTypeReader for &[u8] {
    type OutType = Vec<u8>;

    fn read_buffer(ib: &IpcBuffer, off: &mut usize) -> Self::OutType {
        let len = ib.msg_regs()[*off] as usize;
        let slice = &ib.msg_bytes()[(*off + 1) * REG_LEN..][..len];
        *off += 1 + len.div_ceil(REG_LEN);
        slice.to_vec()
    }
}

impl IpcTypeWriter for &str {
    fn write_buffer(self, ib: &mut IpcBuffer, off: &mut usize) {
        let len = self.len();
        ib.msg_regs_mut()[*off] = len as _;
        ib.msg_bytes_mut()[(*off + 1) * REG_LEN..][..len].copy_from_slice(self.as_bytes());
        *off += 1 + len.div_ceil(REG_LEN);
    }
}

#[macro_export]
macro_rules! read_types {
    ($ib:expr, $($t:ty),*) => {
        {
            let off = &mut 0;
            ($(<$t as $crate::ipcrw::IpcTypeReader>::read_buffer($ib, off)),*)
        }
    };
    ($($t:ty),*) => {{
        let off = &mut 0;
        sel4::with_ipc_buffer_mut(|ib| {
            ($(<$t as $crate::ipcrw::IpcTypeReader>::read_buffer(ib, off)),*)
        })
    }};
}

#[macro_export]
macro_rules! write_values {
    ($ib:expr, $($v:expr),*) => {
        {
            let off = &mut 0;
            $(
                $crate::ipcrw::IpcTypeWriter::write_buffer($v, $ib, off);
            )*
            off.div_ceil($crate::config::REG_LEN)
        }
    };
}

#[macro_export]
macro_rules! reply_with {
    ($ib:expr, $($v:expr),*) => {{
        let wlen = $crate::write_values!($ib, $($v),*);
        sel4::reply($ib, sel4::MessageInfoBuilder::default().length(wlen).build());
    }};
}
