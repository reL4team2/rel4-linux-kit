#![no_std]
#![no_main]

extern crate alloc;

use core::ptr::NonNull;

use common::{
    services::{block::BlockServiceLabel, root::RootService, REG_LEN},
    VIRTIO_MMIO_BLK_VIRT_ADDR,
};
use crate_consts::{DEFAULT_CUSTOM_SLOT, DEFAULT_PARENT_EP, DEFAULT_SERVE_EP, VIRTIO_NET_IRQ};
use sel4::{
    cap::{Endpoint, IrqHandler, Notification},
    init_thread::slot::CNODE,
    with_ipc_buffer, with_ipc_buffer_mut, MessageInfoBuilder,
};
use virtio::HalImpl;
use virtio_drivers::{
    device::blk::{BlkReq, BlkResp, VirtIOBlk},
    transport::mmio::{MmioTransport, VirtIOHeader},
};

mod runtime;
mod virtio;

static ROOT_SERVICE: RootService = RootService::from_bits(DEFAULT_PARENT_EP);

fn main() -> ! {
    common::init_log!(log::LevelFilter::Trace);
    common::init_recv_slot();

    let mut virtio_blk = VirtIOBlk::<HalImpl, MmioTransport>::new(unsafe {
        MmioTransport::new(NonNull::new(VIRTIO_MMIO_BLK_VIRT_ADDR as *mut VirtIOHeader).unwrap())
            .unwrap()
    })
    .expect("[BlockThread] failed to create blk driver");

    log::debug!("Block device capacity: {:#x}", virtio_blk.capacity());

    // Register interrupt handler and notification
    // Allocate irq handler
    let irq_handler = IrqHandler::from_bits(DEFAULT_CUSTOM_SLOT + 1);
    ROOT_SERVICE
        .register_irq(VIRTIO_NET_IRQ as _, CNODE.cap().absolute_cptr(irq_handler))
        .expect("can't register interrupt handler");

    // Allocate notification
    let ntfn = Notification::from_bits(DEFAULT_CUSTOM_SLOT);
    ROOT_SERVICE
        .alloc_notification(CNODE.cap().absolute_cptr(ntfn))
        .expect("Can't register interrupt handler");

    let serve_ep = Endpoint::from_bits(DEFAULT_SERVE_EP);

    irq_handler.irq_handler_set_notification(ntfn).unwrap();
    irq_handler.irq_handler_ack().unwrap();

    // Read block device
    let mut request = BlkReq::default();
    let mut resp = BlkResp::default();
    let mut buffer = [0u8; 512];

    log::debug!("virt queue size: {}", virtio_blk.virt_queue_size());
    log::debug!("Capacity: {:#x}", virtio_blk.capacity() / 2 / 1024);
    for block_id in 0..8 {
        // virtio_blk.read_blocks(block_id, &mut buffer).unwrap();
        unsafe {
            let token = virtio_blk
                .read_blocks_nb(block_id, &mut request, &mut buffer, &mut resp)
                .unwrap();

            log::debug!("token: {}", token);
            log::debug!("peek used: {:?}", virtio_blk.peek_used());

            log::debug!("Waiting for VIRTIO Net IRQ notification");
            sel4::r#yield();
            ntfn.wait();

            irq_handler.irq_handler_ack().unwrap();
            // virtio_blk.ack_interrupt();

            log::debug!("peek used: {:?}", virtio_blk.peek_used());

            // virtio_blk
            //     .complete_read_blocks(token, &request, &mut buffer, &mut resp)
            //     .unwrap();
        };

        log::debug!("Received for VIRTIO Net IRQ notification");
        log::debug!("Get Data Len: {}, 0..4: {:?}", buffer.len(), &buffer[0..4]);
    }

    loop {}

    let rev_msg = MessageInfoBuilder::default();
    loop {
        let (message, _) = serve_ep.recv(());
        let msg_label = match BlockServiceLabel::try_from(message.label()) {
            Ok(label) => label,
            Err(_) => continue,
        };
        log::debug!("Recv <{:?}> len: {}", msg_label, message.length());
        match msg_label {
            BlockServiceLabel::Ping => {
                with_ipc_buffer_mut(|ib| {
                    sel4::reply(ib, rev_msg.build());
                });
            }
            BlockServiceLabel::ReadBlock => {
                let block_id = with_ipc_buffer(|ib| ib.msg_regs()[0]) as _;

                log::debug!("reading block: {}", block_id);
                unsafe {
                    virtio_blk
                        .read_blocks_nb(block_id, &mut request, &mut buffer, &mut resp)
                        .unwrap();
                }
                log::debug!("Waiting for notification");
                ntfn.wait();
                irq_handler.irq_handler_ack().unwrap();
                // virtio_blk.ack_interrupt();
                // slot::CNODE.cap().relative_bits_with_depth(0x22, 64).save_caller().unwrap();
                log::debug!("Block Read: {}", block_id);

                log::debug!("Block Read End");
                with_ipc_buffer_mut(|ib| {
                    ib.msg_bytes_mut()[..buffer.len()].copy_from_slice(&buffer);
                    sel4::reply(ib, rev_msg.length(buffer.len() / REG_LEN).build());
                });
            }
            BlockServiceLabel::WriteBlock => {
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
        }
    }
}
