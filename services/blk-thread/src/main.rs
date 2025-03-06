#![no_std]
#![no_main]

extern crate alloc;

use core::ptr::NonNull;

use common::{
    VIRTIO_MMIO_BLK_VIRT_ADDR,
    consts::REG_LEN,
    services::{
        block::BlockEvent,
        root::{register_irq, register_notify},
        sel4_reply,
    },
};
use crate_consts::{DEFAULT_CUSTOM_SLOT, DEFAULT_SERVE_EP, VIRTIO_NET_IRQ};
use sel4::{
    MessageInfoBuilder,
    cap::{IrqHandler, Notification},
    with_ipc_buffer, with_ipc_buffer_mut,
};
use virtio::HalImpl;
use virtio_drivers::{
    device::blk::{BlkReq, BlkResp, VirtIOBlk},
    transport::mmio::{MmioTransport, VirtIOHeader},
};

mod virtio;

sel4_runtime::entry_point!(main);

fn main() -> ! {
    common::init_log!(log::LevelFilter::Debug);
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
                let block_id = with_ipc_buffer(|ib| {
                    buffer.copy_from_slice(&ib.msg_bytes()[REG_LEN..REG_LEN + 0x200]);
                    ib.msg_regs()[0]
                }) as _;

                let token = unsafe {
                    virtio_blk
                        .write_blocks_nb(block_id, &mut request, &buffer, &mut resp)
                        .unwrap()
                };
                // 顺序不能变，先等待中断，然后处理 virtio_blk 的中断
                // 最后 ACK 中断
                ntfn.wait();
                virtio_blk.ack_interrupt();
                irq_handler.irq_handler_ack().unwrap();

                unsafe {
                    virtio_blk
                        .complete_write_blocks(token, &request, &buffer, &mut resp)
                        .unwrap();
                }
                sel4_reply(rev_msg.build());
            }
            BlockEvent::Capacity => with_ipc_buffer_mut(|ib| {
                ib.msg_regs_mut()[0] = virtio_blk.capacity() * 0x200;
                sel4::reply(ib, rev_msg.length(1).build());
            }),
            // 理论上 AllocPage 需要将任务负责接收内存的一块 IPC 地址 Capability
            // 发送到这个任务中。然后在处理之后填充地址，或者直接写入内存
            BlockEvent::AllocPage | BlockEvent::ReadBlocks | BlockEvent::WriteBlocks => {
                log::error!("unsupperted Operation")
            }
            BlockEvent::Unknown(label) => {
                log::error!("Unknown label: {}", label);
            }
        }
    }
}
