//! Socket 相关的系统调用
//!
//!

use fs::file::File;
use libc_core::socket::{CSocketAddr, SocketAddrIn};
use syscalls::Errno;

use crate::{
    fs::socket::{NetType, Socket},
    syscall::SysResult,
    task::Sel4Task,
};

pub(super) fn sys_socket(
    task: &Sel4Task,
    domain: usize,
    net_type: usize,
    protocol: usize,
) -> SysResult {
    debug!(
        "[task {}] sys_socket @ domain: {:#x}, net_type: {:#x}, protocol: {:#x}",
        task.tid, domain, net_type, protocol
    );

    let net_type = NetType::from_usize(net_type).ok_or(Errno::EINVAL)?;

    let socket = File::new_dev(Socket::new(domain, net_type));
    task.file
        .file_ds
        .lock()
        .add(socket)
        .map_err(|_| Errno::EBADF)
}

pub(super) fn sys_bind(
    task: &Sel4Task,
    socket_fd: usize,
    addr_ptr: *mut SocketAddrIn,
    address_len: usize,
) -> SysResult {
    debug!(
        "[task {}] sys_bind @ socket: {:#x}, addr_ptr: {:p}, address_len: {:#x}",
        task.tid, socket_fd, addr_ptr, address_len
    );
    let socket = task
        .file
        .file_ds
        .lock()
        .get(socket_fd)
        .ok_or(Errno::EINVAL)?
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| Errno::EINVAL)?;
    let socket_addr_bytes = task
        .read_bytes(addr_ptr as _, size_of::<SocketAddrIn>())
        .ok_or(Errno::EINVAL)?;
    let socket_addr = SocketAddrIn::from_bytes(&socket_addr_bytes).clone();
    if socket_addr.family != 0x02 {
        warn!("only support IPV4 now");
        return Err(Errno::EAFNOSUPPORT);
    }

    match socket.net_type {
        NetType::DGRAME => {
            let _ = task
                .file
                .file_ds
                .lock()
                .add_or_replace_at(socket_fd, File::new_dev(socket.clone()));
        }
        NetType::RAW | NetType::STEAM => {}
    }

    Ok(0)
}

pub(super) fn sys_getsockname(
    task: &Sel4Task,
    socket_fd: usize,
    addr_ptr: *mut CSocketAddr,
    len: usize,
) -> SysResult {
    debug!(
        "sys_getsockname @ socket_fd: {:#x}, addr_ptr: {:p}, len: {:#x}",
        socket_fd, addr_ptr, len
    );
    let socket = task
        .file
        .file_ds
        .lock()
        .get(socket_fd)
        .ok_or(Errno::EINVAL)?
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| Errno::EINVAL)?;
    if !addr_ptr.is_null() {
        let socket_bytes = task
            .read_bytes(addr_ptr as _, size_of::<SocketAddrIn>())
            .ok_or(Errno::EINVAL)?;
        let mut socketaddr = SocketAddrIn::from_bytes(&socket_bytes).clone();
        socketaddr.family = 2;
        socketaddr.addr = socket.socketaddr.lock().ip().clone();
        socketaddr.port = socket.socketaddr.lock().port();
        task.write_bytes(addr_ptr as _, socketaddr.as_bytes());
    }
    Ok(0)
}

pub(super) fn sys_sendto(
    task: &Sel4Task,
    socket_fd: usize,
    buffer_ptr: *const u8,
    len: usize,
    flags: usize,
    addr_ptr: *const SocketAddrIn,
    _address_len: usize,
) -> SysResult {
    debug!(
        "[task {}] sys_send @ socket_fd: {:#x}, buffer_ptr: {:p}, len: {:#x}, flags: {:#x} {:p}",
        task.tid, socket_fd, buffer_ptr, len, flags, addr_ptr
    );
    let _socket = task
        .file
        .file_ds
        .lock()
        .get(socket_fd)
        .ok_or(Errno::EINVAL)?
        .get_bare_file()
        .downcast_arc::<Socket>()
        .map_err(|_| Errno::EINVAL)?;

    // if socket.inner.get_local().unwrap().port() == 0 {
    //     socket
    //         .inner
    //         .clone()
    //         .bind(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 0))
    //         .map_err(|_| Errno::EALREADY)?;
    // }

    // let remote = if addr_ptr.is_valid() {
    //     let socket_addr = addr_ptr.read();
    //     Some(SocketAddrV4::new(
    //         socket_addr.addr,
    //         socket_addr.in_port.to_be(),
    //     ))
    // } else {
    //     None
    // };

    // let wlen = socket.inner.sendto(buffer, remote).expect("buffer");
    // Ok(wlen)
    Ok(0)
}
