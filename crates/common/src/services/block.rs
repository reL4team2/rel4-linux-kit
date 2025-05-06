use common_macros::generate_ipc_send;
use num_enum::{FromPrimitive, IntoPrimitive};
use sel4::{MessageInfo, cap::Endpoint};
use slot_manager::LeafSlot;

#[derive(Clone, Copy, Debug, IntoPrimitive, FromPrimitive)]
#[repr(u64)]
pub enum BlockEvent {
    AllocPage,
    Ping,
    Capacity,
    Init,
    ReadBlock,
    WriteBlock,
    ReadBlocks,
    WriteBlocks,
    #[num_enum(catch_all)]
    Unknown(u64),
}

#[derive(Clone, Copy, Debug)]
pub struct BlockService {
    ep: Endpoint,
}

impl BlockService {
    pub const fn from_bits(bits: u64) -> Self {
        Self::new(Endpoint::from_bits(bits))
    }

    pub const fn new(ep: Endpoint) -> Self {
        Self { ep }
    }

    #[generate_ipc_send(label = BlockEvent::Ping)]
    pub fn ping(&self) {}

    #[generate_ipc_send(label = BlockEvent::Init)]
    pub fn init(&self, channel_id: usize) {}

    #[generate_ipc_send(label = BlockEvent::ReadBlock)]
    pub fn read_block(&self, block_id: usize, block_num: usize) -> MessageInfo {}

    #[generate_ipc_send(label = BlockEvent::WriteBlock)]
    pub fn write_block(&self, block_id: usize, block_num: usize) -> MessageInfo {}

    #[generate_ipc_send(label = BlockEvent::Capacity)]
    pub fn capacity(&self) -> u64 {}
}

impl From<LeafSlot> for BlockService {
    fn from(value: LeafSlot) -> Self {
        Self::from_bits(value.raw() as _)
    }
}

impl From<BlockService> for LeafSlot {
    fn from(value: BlockService) -> Self {
        LeafSlot::from_cap(value.ep)
    }
}
