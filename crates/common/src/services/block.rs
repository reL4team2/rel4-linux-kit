use num_enum::{FromPrimitive, IntoPrimitive};
use sel4::{cap::Endpoint, with_ipc_buffer, with_ipc_buffer_mut, MessageInfo, MessageInfoBuilder};
use slot_manager::LeafSlot;

#[derive(Clone, Copy, Debug, IntoPrimitive, FromPrimitive)]
#[repr(u64)]
pub enum BlockEvent {
    Ping,
    ReadBlock,
    WriteBlock,
    #[num_enum(catch_all)]
    Unknown(u64),
}

// FIXME: 公共 patten 就是:
//      1. Label 转换为 message
//      2. Service 的初始化 new from_bits
//      3. Call Message, 甚至 包含 ping?
//
// 其他：可以将 reply 封装为闭包函数
//      reply_ok();
//      reply_error();
//      reply_msg(|ipc_buffer| {} -> msg);
//
// 其他：可以使用类似 Builder 的链式操作构建 MessageBuffer.
//
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

    pub fn read_block(&self, block_id: usize, buffer: &mut [u8]) -> Result<(), ()> {
        with_ipc_buffer_mut(|ipc_buf| {
            ipc_buf.msg_regs_mut()[0] = block_id as _;
        });
        let msg = BlockEvent::ReadBlock.msg().length(1).build();
        // Send and Wait a message
        self.call(msg)?;
        // Copy data from the buffer of the ipc message.
        with_ipc_buffer(|ipc_buf| {
            let len = 0x200;
            buffer[..len].copy_from_slice(&ipc_buf.msg_bytes()[..len]);
        });
        Ok(())
    }

    pub fn write_block(&self, _block_id: usize, _buffer: &[u8]) -> Result<(), ()> {
        unimplemented!("write_blocks")
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
