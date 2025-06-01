#![no_std]
#![no_main]
#![feature(used_with_arg)]

extern crate alloc;

use core::ptr::NonNull;

use common::{
    config::{VIRTIO_MMIO_BLK_VIRT_ADDR, VIRTIO_NET_IRQ},
    services::root::{join_channel, register_irq, register_notify},
    slot::alloc_slot,
};
use flatten_objects::FlattenObjects;
use sel4::cap::{IrqHandler, Notification};
use sel4_runtime::utils::alloc_free_addr;
use srv_gate::{blk::BlockIface, def_blk_impl};
use virtio::HalImpl;
use virtio_drivers::{
    device::blk::{BlkReq, BlkResp, VirtIOBlk},
    transport::mmio::{MmioTransport, VirtIOHeader},
};

mod virtio;

def_blk_impl!(VIRTIOBLK, VirtIOBlkImpl::new(VIRTIO_MMIO_BLK_VIRT_ADDR));

pub struct VirtIOBlkImpl {
    device: VirtIOBlk<HalImpl, MmioTransport>,
    stores: FlattenObjects<(usize, usize), 32>,
    ntfn: Notification,
    irq_handler: IrqHandler,
}

unsafe impl Sync for VirtIOBlkImpl {}
unsafe impl Send for VirtIOBlkImpl {}

impl VirtIOBlkImpl {
    pub fn new(addr: usize) -> Self {
        let ptr = addr as *mut VirtIOHeader;
        let stores = FlattenObjects::<(usize, usize), 32>::new();
        let device = VirtIOBlk::<HalImpl, MmioTransport>::new(unsafe {
            MmioTransport::new(NonNull::new(ptr).unwrap()).unwrap()
        })
        .unwrap();

        // 向 root-task 申请一个中断
        let irq_handler = alloc_slot().cap();
        register_irq(VIRTIO_NET_IRQ as _, irq_handler.into());

        // 向 root-task 申请一个通知
        let ntfn = alloc_slot().cap();
        register_notify(ntfn.into(), 1).expect("Can't register notification");

        // 设置中断信息
        irq_handler.irq_handler_set_notification(ntfn).unwrap();
        irq_handler.irq_handler_ack().unwrap();

        Self {
            device,
            stores,
            ntfn,
            irq_handler,
        }
    }
}

impl BlockIface for VirtIOBlkImpl {
    fn init(&mut self, channel_id: usize) {
        // TODO: 支持多个程序的 channel 初始化，程序如何知道自己的 channel
        let ptr = alloc_free_addr(0) as *mut u8;
        let size = join_channel(channel_id, ptr as usize);
        // self.stores
        //     .add_at(badge as _, (ptr as usize, channel_id))
        //     .map_err(|_| ())
        //     .unwrap();
        self.stores
            .add_at(0, (ptr as usize, channel_id))
            .map_err(|_| ())
            .unwrap();
        alloc_free_addr(size);
    }

    fn read_block(&mut self, block_id: usize, block_num: usize) {
        let mut request = BlkReq::default();
        let mut resp = BlkResp::default();
        // TODO: 根据 badge 分辨多个任务
        let ptr = self.stores.get(0).unwrap().0 as *mut u8;

        let buffer = unsafe { core::slice::from_raw_parts_mut(ptr, 0x200 * block_num) };
        let token = unsafe {
            self.device
                .read_blocks_nb(block_id, &mut request, buffer, &mut resp)
                .unwrap()
        };
        // 顺序不能变，先等待中断，然后处理 virtio_blk 的中断
        // 最后 ACK 中断
        self.ntfn.wait();
        self.device.ack_interrupt();
        self.irq_handler.irq_handler_ack().unwrap();

        unsafe {
            self.device
                .complete_read_blocks(token, &request, buffer, &mut resp)
                .unwrap();
        }
    }

    fn write_block(&mut self, block_id: usize, block_num: usize) {
        let mut request = BlkReq::default();
        let mut resp = BlkResp::default();
        // TODO: 根据 badge 分辨多个任务
        let ptr = self.stores.get(0).unwrap().0 as *mut u8;
        let buffer = unsafe { core::slice::from_raw_parts_mut(ptr, 0x200 * block_num) };

        let token = unsafe {
            self.device
                .write_blocks_nb(block_id, &mut request, buffer, &mut resp)
                .unwrap()
        };
        // 顺序不能变，先等待中断，然后处理 virtio_blk 的中断
        // 最后 ACK 中断
        self.ntfn.wait();
        self.device.ack_interrupt();
        self.irq_handler.irq_handler_ack().unwrap();

        unsafe {
            self.device
                .complete_write_blocks(token, &request, buffer, &mut resp)
                .unwrap();
        }
    }

    fn capacity(&self) -> u64 {
        self.device.capacity() * 0x200
    }
}
