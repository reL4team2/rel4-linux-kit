//! 网络相关结构和函数
//!
//! 主要包含 SmolTCP 相关的实现
use core::net::{Ipv4Addr, SocketAddrV4};

use alloc::{sync::Arc, vec::Vec};
use sel4_kit::arch::current_time;
use smoltcp::{
    iface::{Config, Interface},
    phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken},
    socket::tcp::{Socket, SocketBuffer},
    time::Instant,
    wire::{EthernetAddress, HardwareAddress},
};
use spin::Mutex;

/// 网卡 IP 地址
pub const IP: &str = "10.0.2.15";
/// 网关地质
pub const GW: &str = "10.0.2.2";
/// DNS 服务器
pub const DNS_SEVER: &str = "8.8.8.8";
/// 子网掩码 CIDR 格式
pub const IP_PREFIX: u8 = 24;
/// 数据包最大大小
pub const STANDARD_MTU: usize = 1500;
/// 网卡 MAC 地址
pub const MAC_ADDR: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];

const TCP_RX_BUF_LEN: usize = 64 * 1024;
const TCP_TX_BUF_LEN: usize = 64 * 1024;
// const UDP_RX_BUF_LEN: usize = 64 * 1024;
// const UDP_TX_BUF_LEN: usize = 64 * 1024;

#[allow(dead_code)]
struct NetRxToken<'a>(&'a Arc<Mutex<Vec<Vec<u8>>>>);
#[allow(dead_code)]
struct NetTxToken<'a>(&'a Arc<Mutex<Vec<Vec<u8>>>>);

impl<'a> RxToken for NetRxToken<'a> {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&[u8]) -> R,
    {
        let buffer = self.0.lock().pop().unwrap();
        f(&buffer)
    }
}

impl<'a> TxToken for NetTxToken<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buffer = vec![0u8; len];
        let res = f(&mut buffer);
        self.0.lock().push(buffer);
        res
    }
}

#[allow(dead_code)]
struct DeviceImpl(Arc<Mutex<Vec<Vec<u8>>>>);

impl Device for DeviceImpl {
    type RxToken<'a> = NetRxToken<'a>;
    type TxToken<'a> = NetTxToken<'a>;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        if !self.0.lock().is_empty() {
            Some((NetRxToken(&self.0), NetTxToken(&self.0)))
        } else {
            None
        }
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(NetTxToken(&self.0))
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1514;
        caps.max_burst_size = None;
        caps.medium = Medium::Ethernet;
        caps
    }
}

pub(crate) fn init() {
    let ether_addr = EthernetAddress(MAC_ADDR);
    let tcp_rx_buffer = SocketBuffer::new(vec![0; TCP_RX_BUF_LEN]);
    let tcp_tx_buffer = SocketBuffer::new(vec![0; TCP_TX_BUF_LEN]);
    let mut tcp_socket = Socket::new(tcp_rx_buffer, tcp_tx_buffer);
    let tcp_rx_buffer1 = SocketBuffer::new(vec![0; TCP_RX_BUF_LEN]);
    let tcp_tx_buffer1 = SocketBuffer::new(vec![0; TCP_TX_BUF_LEN]);
    let mut tcp_socket1 = Socket::new(tcp_rx_buffer1, tcp_tx_buffer1);
    let config = Config::new(HardwareAddress::Ethernet(ether_addr));
    // config.random_seed = RANDOM_SEED;

    let mut dev = DeviceImpl(Arc::new(Mutex::new(Vec::new())));
    let inst = Instant::from_micros_const(current_time().as_micros() as _);
    let mut iface = Interface::new(config, &mut dev, inst);

    tcp_socket1
        .listen(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 80))
        .unwrap();

    tcp_socket
        .connect(
            &mut iface.context(),
            SocketAddrV4::new(Ipv4Addr::LOCALHOST, 80),
            SocketAddrV4::new(Ipv4Addr::LOCALHOST, 81),
        )
        .unwrap();
    log::error!("tcp socket status: {:#x?}", tcp_socket.state());

    iface.poll(inst, &mut dev);

    tcp_socket.send_slice(b"Hello World!").unwrap();
    let mut recv_buffer = [0u8; 512];
    let rlen = tcp_socket1.recv_slice(&mut recv_buffer).unwrap();
    log::error!("rlen: {}", rlen);
    log::error!("ether_addr: {:x?}", ether_addr);
    panic!("err")
}
