use crate::__prelude::*;
use common::ipc_trait;

#[ipc_trait(event = BLOCK_EVENT)]
pub trait BlockIface: Sync + Send {
    fn init(&mut self, channel_id: usize);
    fn read_block(&mut self, block_id: usize, block_num: usize);
    fn write_block(&mut self, block_id: usize, block_num: usize);
    fn capacity(&self) -> u64;
}

#[cfg(blk_ipc)]
mod _impl {
    use super::{BlockIface, BlockIfaceEvent};
    use crate::def_blk_impl;
    use common::{generate_ipc_send, root::find_service};
    use sel4::cap::Endpoint;

    def_blk_impl!(BLK_IPC, BlockIfaceIPCImpl {
        ep: find_service("block-thread").unwrap().into(),
    });

    #[derive(Clone, Copy, Debug)]
    pub struct BlockIfaceIPCImpl {
        ep: Endpoint,
    }

    impl BlockIface for BlockIfaceIPCImpl {
        #[generate_ipc_send(label = BlockIfaceEvent::init)]
        fn init(&mut self, channel_id: usize) {
            todo!()
        }

        #[generate_ipc_send(label = BlockIfaceEvent::read_block)]
        fn read_block(&mut self, block_id: usize, block_num: usize) {
            todo!()
        }

        #[generate_ipc_send(label = BlockIfaceEvent::write_block)]
        fn write_block(&mut self, block_id: usize, block_num: usize) {
            todo!()
        }

        #[generate_ipc_send(label = BlockIfaceEvent::capacity)]
        fn capacity(&self) -> u64 {
            todo!()
        }
    }
}
