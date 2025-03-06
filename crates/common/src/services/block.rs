use num_enum::{FromPrimitive, IntoPrimitive};
use sel4::{MessageInfo, MessageInfoBuilder, cap::Endpoint, with_ipc_buffer, with_ipc_buffer_mut};
use slot_manager::LeafSlot;

use crate::consts::REG_LEN;

#[derive(Clone, Copy, Debug, IntoPrimitive, FromPrimitive)]
#[repr(u64)]
pub enum BlockEvent {
    AllocPage,
    Ping,
    Capacity,
    ReadBlock,
    WriteBlock,
    ReadBlocks,
    WriteBlocks,
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

    pub fn write_block(&self, block_id: usize, buffer: &[u8]) -> Result<(), ()> {
        assert_eq!(buffer.len(), 0x200);
        with_ipc_buffer_mut(|ipc_buf| {
            ipc_buf.msg_regs_mut()[0] = block_id as _;
            ipc_buf.msg_bytes_mut()[REG_LEN..REG_LEN + buffer.len()].copy_from_slice(buffer);
        });
        let msg = BlockEvent::WriteBlock
            .msg()
            .length(1 + buffer.len() / REG_LEN)
            .build();
        // Send and Wait a message
        let ret = self.call(msg)?;
        assert_eq!(ret.label(), 0);
        Ok(())
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
