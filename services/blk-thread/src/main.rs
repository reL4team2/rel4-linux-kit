#![no_std]
#![no_main]

extern crate alloc;
extern crate blk_thread;

use blk_thread::VIRTIOBLK;
use common::{config::DEFAULT_SERVE_EP, read_types};
use sel4::{MessageInfoBuilder, with_ipc_buffer_mut};
use sel4_runtime::main;
use srv_gate::blk::BlockIfaceEvent;

#[main]
fn main() {
    let mut virtio_blk = VIRTIOBLK.lock();

    log::debug!("Block device capacity: {:#x}", virtio_blk.capacity());

    let rev_msg = MessageInfoBuilder::default();

    with_ipc_buffer_mut(|ib| {
        loop {
            // TODO: use badge to parse shared memory
            let (msg, _badge) = DEFAULT_SERVE_EP.recv(());
            let msg_label = match BlockIfaceEvent::try_from(msg.label()) {
                Ok(label) => label,
                Err(_) => continue,
            };
            match msg_label {
                BlockIfaceEvent::init => {
                    let channel_id = with_ipc_buffer_mut(|ib| ib.msg_regs()[0] as _);
                    virtio_blk.init(channel_id);
                    sel4::reply(ib, rev_msg.build());
                }
                BlockIfaceEvent::read_block => {
                    let (block_id, block_num) = read_types!(ib, usize, usize);
                    virtio_blk.read_block(block_id, block_num);

                    sel4::reply(ib, rev_msg.build());
                }
                BlockIfaceEvent::write_block => {
                    let (block_id, block_num) = read_types!(ib, usize, usize);
                    virtio_blk.write_block(block_id, block_num);
                    sel4::reply(ib, rev_msg.build());
                }
                BlockIfaceEvent::capacity => {
                    ib.msg_regs_mut()[0] = virtio_blk.capacity();
                    sel4::reply(ib, rev_msg.length(1).build());
                }
            }
        }
    })
}
