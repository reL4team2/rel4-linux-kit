//! IPC for net-thread

use common::NetRequsetabel;
use crate_consts::DEFAULT_CUSTOM_SLOT;
use memory_addr::PAGE_SIZE_4K;
use sel4::{init_thread, with_ipc_buffer_mut, Cap, CapRights, VmAttributes};

use crate::{page_seat_vaddr, OBJ_ALLOCATOR};

fn send_net_ipc(label: NetRequsetabel, cap: Option<Cap<sel4::cap_type::SmallPage>>) {
    let ipc_ep = Cap::<sel4::cap_type::Endpoint>::from_bits(DEFAULT_CUSTOM_SLOT + 2);
    with_ipc_buffer_mut(|buffer| {
        if let Some(cap) = cap {
            buffer.caps_or_badges_mut()[0] = cap.bits() as _;
        }
    });
    ipc_ep.call(label.build());
}

/// Generate capabilities to access item in other task
fn gen_cap<T: Sized + Copy>(item: *const T, num: Option<usize>) -> Cap<sel4::cap_type::SmallPage> {
    let new_cap = OBJ_ALLOCATOR
        .lock()
        .allocate_and_retyped_fixed_sized::<sel4::cap_type::Granule>();
    new_cap
        .frame_map(
            init_thread::slot::VSPACE.cap(),
            page_seat_vaddr(),
            CapRights::all(),
            VmAttributes::default(),
        )
        .unwrap();
    let copy_num = num.unwrap_or(1);

    if core::mem::size_of::<T>() * copy_num > PAGE_SIZE_4K {
        panic!("Item size is too large");
    }
    unsafe {
        core::ptr::copy_nonoverlapping(
            item as *const T,
            page_seat_vaddr() as *mut T,
            core::mem::size_of::<T>() * copy_num,
        );
    }
    new_cap
}

pub(crate) type TCPSocketId = u64;

#[allow(unused)]
pub(crate) mod tcp {
    use core::net::SocketAddr;

    use axerrno::AxResult;
    use sel4::{debug_println, with_ipc_buffer};

    use super::*;

    fn handle_axresult(val: u64) -> AxResult<usize> {
        let val_i32 = val as i32;
        match val_i32 {
            0 => Ok(val as usize),
            _ => Err(val_i32.try_into().unwrap()),
        }
    }

    pub(crate) fn new() -> TCPSocketId {
        send_net_ipc(NetRequsetabel::New, None);
        with_ipc_buffer(|buffer| buffer.msg_regs()[0])
    }

    pub(crate) fn is_non_blocking(socket_id: TCPSocketId) -> bool {
        send_net_ipc(NetRequsetabel::IsNonBlocking(socket_id), None);
        with_ipc_buffer(|buffer| buffer.msg_regs()[0] != 0)
    }

    pub(crate) fn set_nonblocking(socket_id: TCPSocketId, is_nonblocking: bool) {
        send_net_ipc(
            NetRequsetabel::SetNonBlocking(socket_id, is_nonblocking as u64),
            None,
        );
    }

    pub(crate) fn bind(socket_id: TCPSocketId, local_addr: SocketAddr) -> AxResult {
        let cap = gen_cap(&local_addr, None);
        send_net_ipc(
            NetRequsetabel::Bind(socket_id, &local_addr as *const SocketAddr as u64),
            Some(cap),
        );
        with_ipc_buffer(|buffer| handle_axresult(buffer.msg_regs()[0]).map(|_| ()))
    }

    pub(crate) fn send(socket_id: TCPSocketId, buf: &[u8]) -> AxResult<usize> {
        if buf.len() > PAGE_SIZE_4K {
            panic!("The buffer is not contained in a page.");
        }
        let cap = gen_cap(buf.as_ptr(), Some(buf.len()));
        unsafe {
            core::ptr::copy_nonoverlapping(buf.as_ptr(), page_seat_vaddr() as *mut u8, buf.len());
        }
        send_net_ipc(
            NetRequsetabel::Send(socket_id, buf.as_ptr() as u64, buf.len() as u64),
            Some(cap),
        );
        with_ipc_buffer(|buffer| handle_axresult(buffer.msg_regs()[0]))
    }

    pub(crate) fn recv(socket_id: TCPSocketId, buf: &mut [u8]) -> AxResult<usize> {
        if buf.len() > PAGE_SIZE_4K {
            panic!("The buffer is not contained in a page.");
        }
        let cap = gen_cap(buf.as_ptr(), Some(buf.len()));
        send_net_ipc(
            NetRequsetabel::Recv(socket_id, buf.as_ptr() as u64, buf.len() as u64),
            Some(cap),
        );
        with_ipc_buffer(|buffer| {
            let len = buffer.msg_regs()[0] as usize;
            unsafe {
                core::ptr::copy_nonoverlapping(
                    page_seat_vaddr() as *const u8,
                    buf.as_mut_ptr(),
                    len,
                );
            }
            handle_axresult(buffer.msg_regs()[0])
        })
    }

    pub(crate) fn recv_timeout(
        socket_id: TCPSocketId,
        buf: &mut [u8],
        timeout: u64,
    ) -> AxResult<usize> {
        if buf.len() > PAGE_SIZE_4K {
            panic!("The buffer is not contained in a page.");
        }
        let cap = gen_cap(buf.as_ptr(), Some(buf.len()));
        send_net_ipc(
            NetRequsetabel::RecvTimeout(socket_id, buf.as_ptr() as u64, buf.len() as u64, timeout),
            Some(cap),
        );
        with_ipc_buffer(|buffer| {
            let len = buffer.msg_regs()[0] as usize;
            unsafe {
                core::ptr::copy_nonoverlapping(
                    page_seat_vaddr() as *const u8,
                    buf.as_mut_ptr(),
                    len,
                );
            }
            handle_axresult(buffer.msg_regs()[0])
        })
    }

    pub(crate) fn listen(socket_id: TCPSocketId) -> AxResult {
        send_net_ipc(NetRequsetabel::Listen(socket_id), None);
        with_ipc_buffer(|buffer| handle_axresult(buffer.msg_regs()[0]).map(|_| ()))
    }

    pub(crate) fn connect(socket_id: TCPSocketId, remote_addr: SocketAddr) -> AxResult {
        let cap = gen_cap(&remote_addr, None);
        send_net_ipc(NetRequsetabel::Connect(socket_id, 0), Some(cap));
        with_ipc_buffer(|buffer| handle_axresult(buffer.msg_regs()[0]).map(|_| ()))
    }

    pub(crate) fn accept(socket_id: TCPSocketId) -> AxResult<[u64; 5]> {
        send_net_ipc(NetRequsetabel::Accept(socket_id), None);
        with_ipc_buffer(|buffer| {
            if (buffer.msg_regs()[0] as i64) < 0 {
                let error_code = (buffer.msg_regs()[0] as i32).abs();
                Err(error_code.try_into().unwrap())
            } else {
                Ok([
                    buffer.msg_regs()[0],
                    buffer.msg_regs()[1],
                    buffer.msg_regs()[2],
                    buffer.msg_regs()[3],
                    buffer.msg_regs()[4],
                ])
            }
        })
    }

    pub(crate) fn close(socket_id: TCPSocketId) -> AxResult {
        send_net_ipc(NetRequsetabel::Close(socket_id), None);
        with_ipc_buffer(|buffer| handle_axresult(buffer.msg_regs()[0]).map(|_| ()))
    }

    pub(crate) fn shutdown(socket_id: TCPSocketId) -> AxResult {
        send_net_ipc(NetRequsetabel::Shutdown(socket_id), None);
        with_ipc_buffer(|buffer| handle_axresult(buffer.msg_regs()[0]).map(|_| ()))
    }
}
