#![no_std]
#![no_main]

extern crate alloc;

use core::ptr::NonNull;

use common::{
    consts::DEFAULT_SERVE_EP,
    services::{
        block::BlockEvent,
        root::{join_channel, register_irq, register_notify},
    },
};
use config::{DEFAULT_CUSTOM_SLOT, VIRTIO_MMIO_BLK_VIRT_ADDR, VIRTIO_NET_IRQ};
use flatten_objects::FlattenObjects;
use sel4::{
    MessageInfoBuilder,
    cap::{IrqHandler, Notification},
    with_ipc_buffer_mut,
};
use sel4_runtime::utils::alloc_free_addr;
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
    // let mut buffer = [0u8; 0x1000];

    let mut stores = FlattenObjects::<(usize, usize), 32>::new();
    let rev_msg = MessageInfoBuilder::default();

    with_ipc_buffer_mut(|ib| {
        loop {
            let (message, badge) = DEFAULT_SERVE_EP.recv(());
            match BlockEvent::from(message.label()) {
                BlockEvent::Ping => sel4::reply(ib, rev_msg.build()),
                BlockEvent::Init => {
                    let ptr = alloc_free_addr(0) as *mut u8;
                    assert_eq!(message.length(), 1);
                    let channel_id = with_ipc_buffer_mut(|ib| ib.msg_regs()[0] as _);
                    let size = join_channel(channel_id, ptr as usize).unwrap();
                    stores
                        .add_at(badge as _, (ptr as usize, channel_id))
                        .map_err(|_| ())
                        .unwrap();
                    sel4::reply(ib, rev_msg.build());
                    alloc_free_addr(size);
                }
                BlockEvent::ReadBlock => {
                    let ptr = stores.get(badge as usize).unwrap().0 as *mut u8;
                    let block_id = ib.msg_regs()[0] as _;
                    let block_num = ib.msg_regs()[1] as usize;

                    let buffer = unsafe { core::slice::from_raw_parts_mut(ptr, 0x200 * block_num) };
                    let token = unsafe {
                        virtio_blk
                            .read_blocks_nb(block_id, &mut request, buffer, &mut resp)
                            .unwrap()
                    };
                    // 顺序不能变，先等待中断，然后处理 virtio_blk 的中断
                    // 最后 ACK 中断
                    ntfn.wait();
                    virtio_blk.ack_interrupt();
                    irq_handler.irq_handler_ack().unwrap();

                    unsafe {
                        virtio_blk
                            .complete_read_blocks(token, &request, buffer, &mut resp)
                            .unwrap();
                    }
                    sel4::reply(ib, rev_msg.build());
                }
                BlockEvent::WriteBlock => {
                    let ptr = stores.get(badge as usize).unwrap().0 as *mut u8;
                    let (block_id, block_num) = (ib.msg_regs()[0] as _, ib.msg_regs()[1] as usize);
                    let buffer = unsafe { core::slice::from_raw_parts_mut(ptr, 0x200 * block_num) };

                    let token = unsafe {
                        virtio_blk
                            .write_blocks_nb(block_id, &mut request, buffer, &mut resp)
                            .unwrap()
                    };
                    // 顺序不能变，先等待中断，然后处理 virtio_blk 的中断
                    // 最后 ACK 中断
                    ntfn.wait();
                    virtio_blk.ack_interrupt();
                    irq_handler.irq_handler_ack().unwrap();

                    unsafe {
                        virtio_blk
                            .complete_write_blocks(token, &request, buffer, &mut resp)
                            .unwrap();
                    }
                    sel4::reply(ib, rev_msg.build());
                }
                BlockEvent::Capacity => {
                    ib.msg_regs_mut()[0] = virtio_blk.capacity() * 0x200;
                    sel4::reply(ib, rev_msg.length(1).build());
                }
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
    })
}
