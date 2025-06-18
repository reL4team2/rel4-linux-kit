#![allow(missing_docs)]
#![allow(warnings)]
use alloc::{sync::Arc, vec::Vec};
use core::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use fs::INodeInterface;
use libc_core::{
    poll::PollEvent,
    types::{Stat, StatMode},
};
use spin::Mutex;
use vfscore::VfsResult;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NetType {
    STEAM,
    DGRAME,
    RAW,
}

impl NetType {
    pub fn from_usize(value: usize) -> Option<Self> {
        match value {
            1 => Some(Self::STEAM),
            2 => Some(Self::DGRAME),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct SocketOptions {
    pub wsize: usize,
    pub rsize: usize,
}

#[allow(dead_code)]
pub struct Socket {
    pub domain: usize,
    pub net_type: NetType,
    pub socketaddr: Mutex<SocketAddrV4>,
    pub options: Mutex<SocketOptions>,
    pub buf: Mutex<Vec<u8>>,
}

unsafe impl Sync for Socket {}
unsafe impl Send for Socket {}

impl Socket {
    pub fn new(domain: usize, net_type: NetType) -> Arc<Self> {
        log::error!("open net type: {:?}", net_type);
        Arc::new(Self {
            domain,
            net_type,
            socketaddr: Mutex::new(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)),
            options: Mutex::new(SocketOptions { wsize: 0, rsize: 0 }),
            buf: Mutex::new(vec![]),
        })
    }

    pub fn recv_from(&self) -> VfsResult<(Vec<u8>, SocketAddrV4)> {
        // log::warn!("try to recv data from {}", self.inner.get_local().unwrap());
        // match self.inner.recv_from() {
        //     Ok((data, remote)) => Ok((data, remote)),
        //     Err(_err) => Err(Errno::EWOULDBLOCK),
        // }
        panic!("recv from");
    }

    pub fn new_with_inner(domain: usize, net_type: NetType) -> Arc<Self> {
        Arc::new(Self {
            domain,
            net_type,
            socketaddr: Mutex::new(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0).into()),
            options: Mutex::new(SocketOptions { wsize: 0, rsize: 0 }),
            buf: Mutex::new(vec![]),
        })
    }

    pub fn reuse(&self, port: u16) -> Self {
        // NET_SERVER.get_tcp(port)
        // match self.inner.get_protocol().unwrap() {
        //     lose_net_stack::connection::SocketType::TCP => {
        //         if let Some(socket_inner) = NET_SERVER.get_tcp(&port) {
        //             Self {
        //                 domain: self.domain,
        //                 net_type: self.net_type,
        //                 inner: socket_inner,
        //                 options: Mutex::new(self.options.lock().clone()),
        //                 buf: Mutex::new(vec![]),
        //             }
        //         } else {
        //             unreachable!("can't reusetcp in blank tcp")
        //         }
        //     }
        //     lose_net_stack::connection::SocketType::UDP => {
        //         if let Some(socket_inner) = NET_SERVER.get_udp(&port) {
        //             Self {
        //                 domain: self.domain,
        //                 net_type: self.net_type,
        //                 inner: socket_inner,
        //                 options: Mutex::new(self.options.lock().clone()),
        //                 buf: Mutex::new(vec![]),
        //             }
        //         } else {
        //             unreachable!("can't reusetcp in blank udp")
        //         }
        //     }
        //     lose_net_stack::connection::SocketType::RAW => todo!(),
        // }
        panic!("reuse port {}", port)
    }
}

impl INodeInterface for Socket {
    fn readat(&self, _offset: usize, _buffer: &mut [u8]) -> VfsResult<usize> {
        // let mut data = self.buf.lock().clone();
        // let rlen;
        // if buf.len() > 0 {
        //     rlen = cmp::min(buf.len(), buffer.len());
        //     let
        // } else {
        //     rlen = cmp::min(data.len(), buffer.len());
        //     buffer[..rlen].copy_from_slice(&data[..rlen]);
        //     self.options.lock().rsize += rlen;
        //     if rlen < data.len() {

        //     }
        // }
        // Ok(rlen)
        // if data.len() == 0 {
        //     match self.inner.recv_from() {
        //         Ok((recv_data, _)) => {
        //             data = recv_data;
        //         }
        //         Err(_err) => return Err(Errno::EWOULDBLOCK),
        //     }
        // }
        // let rlen = cmp::min(data.len(), buffer.len());
        // buffer[..rlen].copy_from_slice(&data[..rlen]);
        // self.options.lock().rsize += rlen;
        // if buffer.len() == 1 {
        //     DebugConsole::putchar(buffer[0]);
        // }
        // if rlen < data.len() {
        //     *self.buf.lock() = data[rlen..].to_vec();
        // } else {
        //     self.buf.lock().clear();
        // }
        // Ok(rlen)
        Ok(0)
    }

    fn writeat(&self, _offset: usize, _buffer: &[u8]) -> VfsResult<usize> {
        // match self.inner.sendto(&buffer, None) {
        //     Ok(len) => {
        //         self.options.lock().wsize += len;
        //         Ok(len)
        //     }
        //     Err(_err) => Err(Errno::EPERM),
        // }
        Ok(0)
    }

    fn poll(&self, events: PollEvent) -> VfsResult<PollEvent> {
        // let mut res = PollEvent::NONE;
        // if events.contains(PollEvent::OUT)
        //     && !self.inner.is_closed().unwrap()
        //     && self.inner.get_remote().is_ok()
        // {
        //     res |= PollEvent::OUT;
        // }
        // if self.inner.readable().unwrap() && events.contains(PollEvent::IN) {
        //     res |= PollEvent::IN;
        // }
        // Ok(res)
        Ok(events)
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        stat.mode = StatMode::SOCKET;
        Ok(())
    }
}
