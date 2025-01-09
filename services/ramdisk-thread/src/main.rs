#![no_std]
#![no_main]

extern crate alloc;

use common::services::{block::BlockServiceLabel, root::RootService, REG_LEN};
use crate_consts::{DEFAULT_PARENT_EP, DEFAULT_SERVE_EP};
use sel4::{cap::Endpoint, with_ipc_buffer, with_ipc_buffer_mut, MessageInfoBuilder};

mod runtime;

const DISK_FILE: &[u8] = include_bytes!("../../../mount.img");
const BLOCK_SIZE: usize = 4096;
pub struct Ext4Disk;

static ROOT_SERVICE: RootService = RootService::from_bits(DEFAULT_PARENT_EP);

fn main() -> ! {
    common::init_log!(log::LevelFilter::Trace);
    common::init_recv_slot();

    log::debug!("Starting...");

    let serve_ep = Endpoint::from_bits(DEFAULT_SERVE_EP);

    let rev_msg = MessageInfoBuilder::default();
    loop {
        let (message, _) = serve_ep.recv(());
        let msg_label = match BlockServiceLabel::try_from(message.label()) {
            Ok(label) => label,
            Err(_) => continue,
        };
        match msg_label {
            BlockServiceLabel::Ping => {
                with_ipc_buffer_mut(|ib| {
                    sel4::reply(ib, rev_msg.build());
                });
            }
            BlockServiceLabel::ReadBlock => {
                let block_id: usize = with_ipc_buffer(|ib| ib.msg_regs()[0]) as _;

                with_ipc_buffer_mut(|ib| {
                    let rlen = 0x200;
                    let disk_start = block_id * 0x200;
                    let disk_slice = &DISK_FILE[disk_start..disk_start + rlen];
                    ib.msg_bytes_mut()[..rlen].copy_from_slice(disk_slice);
                    sel4::reply(ib, rev_msg.length(rlen / REG_LEN).build());
                });
            }
            BlockServiceLabel::WriteBlock => {
                unimplemented!("Write Block Operation")
            }
        }
    }
}
