#![no_std]
#![no_main]

extern crate alloc;
extern crate blk_thread;

use blk_thread::VIRTIOBLK;
use common::{config::DEFAULT_SERVE_EP, read_types, reply_with};
use sel4::{MessageInfoBuilder, with_ipc_buffer_mut};
use sel4_runtime::main;
use srv_gate::blk::BlockIfaceEvent;

sel4_runtime::define_heap!(common::config::SERVICE_HEAP_SIZE);

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
                    let channel_id = read_types!(ib, usize);
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
                    reply_with!(ib, virtio_blk.capacity())
                }
            }
        }
    })
}
