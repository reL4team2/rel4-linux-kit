use core::net::{Ipv4Addr, SocketAddr};

use alloc::vec::Vec;
use axerrno::AxError;
use common::LibcSocketAddr;
use syscalls::Errno;

use crate::{
    child_test::TASK_MAP,
    syscall::SysResult,
    utils::{read_item, read_item_list, write_item, write_item_list},
};

use super::ipc::tcp;

pub fn sys_socket(_badge: u64, _domain: usize, _type: usize, _protocol: usize) -> SysResult {
    Ok(tcp::new() as usize)
}

pub fn sys_bind(
    badge: u64,
    socket_fd: i32,
    addr: *const LibcSocketAddr,
    _addr_len: u32,
) -> SysResult {
    let task_map = TASK_MAP.lock();
    let task = task_map.get(&badge).unwrap();
    let addr = read_item(task, addr)?;
    let socket_id = socket_fd as u64;
    let local_addr: SocketAddr = addr.into();
    match tcp::bind(socket_id, local_addr) {
        Ok(()) => Ok(0),
        Err(AxError::InvalidInput) => Err(Errno::EINVAL),
        Err(_) => panic!("Unknown Error!"),
    }
}

pub fn sys_connect(
    badge: u64,
    socket_fd: i32,
    addr: *const LibcSocketAddr,
    _addr_len: u32,
) -> SysResult {
    let task_map = TASK_MAP.lock();
    let task = task_map.get(&badge).unwrap();
    let addr = read_item(task, addr)?;
    let socket_id = socket_fd as u64;
    let remote_addr: SocketAddr = addr.into();
    match tcp::connect(socket_id, remote_addr) {
        Ok(()) => Ok(0),
        Err(AxError::InvalidInput) | Err(AxError::AddrInUse) => Err(Errno::EINVAL),
        Err(_) => panic!("Unknown Error!"),
    }
}

pub fn sys_listen(_badge: u64, socket_fd: i32) -> SysResult {
    let socket_id = socket_fd as u64;
    match tcp::listen(socket_id) {
        Ok(()) => Ok(0),
        Err(AxError::InvalidInput) | Err(AxError::AddrInUse) => Err(Errno::EINVAL),
        Err(_) => panic!("Unknown Error!"),
    }
}

pub fn sys_accept(
    badge: u64,
    socket_fd: i32,
    addr: *mut LibcSocketAddr,
    _addr_len: u32,
) -> SysResult {
    let task_map = TASK_MAP.lock();
    let task = task_map.get(&badge).unwrap();
    fn parse_ipaddr(is_ipv4: bool, addr_low: u64, addr_high: u64, port: u16) -> SocketAddr {
        if is_ipv4 {
            let addr = addr_low.to_be_bytes();
            SocketAddr::new(
                core::net::IpAddr::V4(Ipv4Addr::new(addr[0], addr[1], addr[2], addr[3])),
                port,
            )
        } else {
            let addr = (addr_high as u128) << 32 | addr_low as u128;
            let addr = addr.to_be_bytes();
            SocketAddr::new(core::net::IpAddr::V6(addr.into()), port)
        }
    }
    let socket_id = socket_fd as u64;
    match tcp::accept(socket_id) {
        Ok(ans) => {
            let socket_id = ans[0] as usize;
            let is_ipv4 = ans[1] != 0;
            let port = ans[2] as u16;
            let socket_addr = parse_ipaddr(is_ipv4, ans[3], ans[4], port);
            write_item(task, addr, &socket_addr.into());
            Ok(socket_id)
        }
        Err(AxError::InvalidInput) | Err(AxError::AddrInUse) => Err(Errno::EINVAL),
        Err(_) => panic!("Unknown Error!"),
    }
}

pub fn sys_shutdown(_badge: u64, socket_fd: i32, _how: i32) -> SysResult {
    let socket_id = socket_fd as u64;
    match tcp::shutdown(socket_id) {
        Ok(()) => Ok(0),
        Err(AxError::InvalidInput) | Err(AxError::AddrInUse) => Err(Errno::EINVAL),
        Err(_) => panic!("Unknown Error!"),
    }
}

pub fn sys_sendto(
    badge: u64,
    socket_fd: i32,
    buf: *const u8,
    len: usize,
    _flags: i32,
    _addr: *const LibcSocketAddr,
    _addr_len: usize,
) -> SysResult {
    let task_map = TASK_MAP.lock();
    let task = task_map.get(&badge).unwrap();
    let socket_id = socket_fd as u64;
    let _remote_addr: SocketAddr = read_item(task, _addr)?.into();
    // TODO: copy the capabilities of the user thread and transmit it directly
    let mut payload = Vec::with_capacity(len);
    read_item_list(task, buf, Some(len), payload.as_mut_slice());
    match tcp::send(socket_id, payload.as_slice()) {
        Ok(len) => Ok(len),
        Err(AxError::InvalidInput) | Err(AxError::AddrInUse) => Err(Errno::EINVAL),
        Err(_) => panic!("Unknown Error!"),
    }
}

pub fn sys_recvfrom(
    badge: u64,
    socket_fd: i32,
    buf: *mut u8,
    len: usize,
    _flags: i32,
    _addr: *const LibcSocketAddr,
    _addr_len: usize,
) -> SysResult {
    let task_map = TASK_MAP.lock();
    let task = task_map.get(&badge).unwrap();
    let socket_id = socket_fd as u64;
    let mut recv_buf = Vec::with_capacity(len);
    match tcp::recv(socket_id, recv_buf.as_mut_slice()) {
        Ok(len) => write_item_list(task, buf, Some(len), recv_buf.as_slice()),
        Err(AxError::InvalidInput) | Err(AxError::AddrInUse) => Err(Errno::EINVAL),
        Err(_) => panic!("Unknown Error!"),
    }
}
