use num_enum::{FromPrimitive, IntoPrimitive};
use sel4::{MessageInfo, MessageInfoBuilder, cap::Endpoint, with_ipc_buffer, with_ipc_buffer_mut};
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

impl BlockEvent {
    fn msg(&self) -> MessageInfoBuilder {
        MessageInfoBuilder::default().label((*self).into())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BlockService {
    ep_cap: Endpoint,
}

impl BlockService {
    pub const fn from_bits(bits: u64) -> Self {
        Self::new(Endpoint::from_bits(bits))
    }

    pub const fn new(ep_cap: Endpoint) -> Self {
        Self { ep_cap }
    }

    pub fn call(&self, msg: MessageInfo) -> Result<MessageInfo, ()> {
        let msg = self.ep_cap.call(msg);
        if msg.label() != 0 {
            return Err(());
        }
        Ok(msg)
    }

    pub fn ping(&self) -> Result<MessageInfo, ()> {
        let ping_msg = BlockEvent::Ping.msg().build();
        self.call(ping_msg)
    }

    pub fn init(&self, channel_id: usize) -> Result<(), ()> {
        with_ipc_buffer_mut(|ib| {
            ib.msg_regs_mut()[0] = channel_id as _;
        });
        let msg = BlockEvent::Init.msg().length(1).build();
        let ret = self.call(msg)?;
        assert_eq!(ret.label(), 0);

        Ok(())
    }

    pub fn read_block(&self, block_id: usize, block_num: usize) -> Result<MessageInfo, ()> {
        with_ipc_buffer_mut(|ipc_buf| {
            ipc_buf.msg_regs_mut()[0] = block_id as _;
            ipc_buf.msg_regs_mut()[1] = block_num as _;
        });
        let msg = BlockEvent::ReadBlock.msg().length(2).build();
        // Send and Wait a message
        self.call(msg)
    }

    pub fn write_block(&self, block_id: usize, block_num: usize) -> Result<MessageInfo, ()> {
        with_ipc_buffer_mut(|ipc_buf| {
            ipc_buf.msg_regs_mut()[0] = block_id as _;
            ipc_buf.msg_regs_mut()[1] = block_num as _;
        });
        let msg = BlockEvent::WriteBlock.msg().length(2).build();
        let ret = self.call(msg)?;
        assert_eq!(ret.label(), 0);
        Ok(ret)
    }

    pub fn capacity(&self) -> Result<u64, ()> {
        let msg = BlockEvent::Capacity.msg().length(1).build();
        let ret = self.call(msg)?;
        assert_eq!(ret.label(), 0);

        Ok(with_ipc_buffer(|ib| ib.msg_regs()[0]))
    }
}

impl From<LeafSlot> for BlockService {
    fn from(value: LeafSlot) -> Self {
        Self::from_bits(value.raw() as _)
    }
}

impl From<BlockService> for LeafSlot {
    fn from(value: BlockService) -> Self {
        LeafSlot::from_cap(value.ep_cap)
    }
}
