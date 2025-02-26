#![no_std]
#![no_main]

extern crate alloc;

use core::ptr::NonNull;

use common::{
    consts::REG_LEN,
    services::{
        block::BlockEvent,
        root::{register_irq, register_notify},
    },
    VIRTIO_MMIO_BLK_VIRT_ADDR,
};
use crate_consts::{DEFAULT_CUSTOM_SLOT, DEFAULT_SERVE_EP, VIRTIO_NET_IRQ};
use sel4::{
    cap::{IrqHandler, Notification},
    with_ipc_buffer, with_ipc_buffer_mut, MessageInfoBuilder,
};
use virtio::HalImpl;
use virtio_drivers::{
    device::blk::{BlkReq, BlkResp, VirtIOBlk},
    transport::mmio::{MmioTransport, VirtIOHeader},
};

mod runtime;
mod virtio;

fn main() -> ! {
    common::init_log!(log::LevelFilter::Error);
    common::init_recv_slot();

    let mut virtio_blk = VirtIOBlk::<HalImpl, MmioTransport>::new(unsafe {
        MmioTransport::new(NonNull::new(VIRTIO_MMIO_BLK_VIRT_ADDR as *mut VirtIOHeader).unwrap())
            .unwrap()
    })
    .expect("[BlockThread] failed to create blk driver");

    log::debug!("Block device capacity: {:#x}", virtio_blk.capacity());

    // 向 root-task 申请一个中断
    let irq_handler = IrqHandler::from_bits(DEFAULT_CUSTOM_SLOT + 1);
    register_irq(VIRTIO_NET_IRQ as _, irq_handler.into()).expect("Can't register irq handler");

    // 向 root-task 申请一个通知
    let ntfn = Notification::from_bits(DEFAULT_CUSTOM_SLOT);
    register_notify(ntfn.into(), 1).expect("Can't register notification");

    // 设置中断信息
    irq_handler.irq_handler_set_notification(ntfn).unwrap();
    irq_handler.irq_handler_ack().unwrap();

    let mut request = BlkReq::default();
    let mut resp = BlkResp::default();
    let mut buffer = [0u8; 512];

    let rev_msg = MessageInfoBuilder::default();
    loop {
        let (message, _) = DEFAULT_SERVE_EP.recv(());
        match BlockEvent::from(message.label()) {
            BlockEvent::Ping => {
                with_ipc_buffer_mut(|ib| {
                    sel4::reply(ib, rev_msg.build());
                });
            }
            BlockEvent::ReadBlock => {
                let block_id = with_ipc_buffer(|ib| ib.msg_regs()[0]) as _;

                let token = unsafe {
                    virtio_blk
                        .read_blocks_nb(block_id, &mut request, &mut buffer, &mut resp)
                        .unwrap()
                };
                // 顺序不能变，先等待中断，然后处理 virtio_blk 的中断
                // 最后 ACK 中断
                ntfn.wait();
                virtio_blk.ack_interrupt();
                irq_handler.irq_handler_ack().unwrap();

                unsafe {
                    virtio_blk
                        .complete_read_blocks(token, &request, &mut buffer, &mut resp)
                        .unwrap();
                }

                with_ipc_buffer_mut(|ib| {
                    ib.msg_bytes_mut()[..buffer.len()].copy_from_slice(&buffer);
                    sel4::reply(ib, rev_msg.length(buffer.len() / REG_LEN).build());
                });
            }
            BlockEvent::WriteBlock => {
                unimplemented!("Write Block Operation")
                // let (block_id, wlen) = with_ipc_buffer_mut(|ib| {
                //     let regs = ib.msg_regs();
                //     (regs[0] as _, regs[1] as _)
                // });
                // self.write_blocks(block_id, &buffer[..wlen]);
                // with_ipc_buffer_mut(|ib| {
                //     sel4::reply(ib, rev_msg.build());
                // });
            }
            BlockEvent::Unknown(label) => {
                log::error!("Unknown label: {}", label);
            }
        }
    }
}
