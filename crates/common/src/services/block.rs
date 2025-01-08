use num_enum::{IntoPrimitive, TryFromPrimitive};
use sel4::{
    cap::Endpoint, with_ipc_buffer, with_ipc_buffer_mut, MessageInfo, MessageInfoBuilder, Word,
};

#[derive(Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u64)]
pub enum BlockServiceLabel {
    Ping,
    ReadBlock,
    WriteBlock,
}

pub struct BlockService {
    ep_cap: Endpoint,
}

impl BlockService {
    pub const fn new(ep_cap: Endpoint) -> Self {
        Self { ep_cap }
    }

    pub fn call(&self, msg: MessageInfo) -> Result<MessageInfo, ()> {
        Ok(self.ep_cap.call(msg))
    }

    pub fn ping(&self) -> Result<MessageInfo, ()> {
        let ping_msg = MessageInfoBuilder::default()
            .label(BlockServiceLabel::Ping.into())
            .build();
        self.call(ping_msg)
    }

    pub fn read_block(&self, block_id: usize, buffer: &mut [u8]) -> Result<(), ()> {
        with_ipc_buffer_mut(|ipc_buf| {
            ipc_buf.msg_regs_mut()[0] = block_id as _;
        });
        let msg = MessageInfoBuilder::default()
            .label(BlockServiceLabel::ReadBlock.into())
            .length(1)
            .build();
        log::debug!("read block start");
        // Send and Wait a message
        let msg = self.call(msg)?;
        log::debug!("read block reply: {:?}", msg);
        // Copy data from the buffer of the ipc message.
        with_ipc_buffer(|ipc_buf| {
            let len = ipc_buf.user_data() as usize;
            buffer[..len].copy_from_slice(&ipc_buf.msg_bytes()[..len]);
        });
        Ok(())
    }

    pub fn write_block(&self, _block_id: usize, _buffer: &[u8]) -> Result<(), ()> {
        unimplemented!("write_blocks")
    }
}

pub trait BlockServiceAdapter {
    fn read_blocks(&mut self, block_id: usize, buffer: &mut [u8]) -> usize;
    fn write_blocks(&mut self, block_id: usize, buffer: &[u8]) -> usize;

    fn handle(&mut self, ep: Endpoint) {
        let rev_msg = MessageInfoBuilder::default();
        let mut buffer = [0u8; 0x200];
        loop {
            let (msg, _) = ep.recv(());
            let label = match BlockServiceLabel::try_from(msg.label()) {
                Ok(v) => v,
                _ => continue,
            };
            match label {
                BlockServiceLabel::Ping => {
                    with_ipc_buffer_mut(|ipc_buffer| {
                        sel4::reply(ipc_buffer, rev_msg.build());
                    });
                }
                BlockServiceLabel::ReadBlock => {
                    let block_id = with_ipc_buffer(|ib| ib.msg_regs()[0]) as _;
                    let len = self.read_blocks(block_id, &mut buffer);
                    with_ipc_buffer_mut(|ib| {
                        ib.msg_bytes_mut()[..len].copy_from_slice(&buffer[..len]);
                        sel4::reply(ib, rev_msg.length(len / size_of::<Word>()).build());
                    });
                }
                BlockServiceLabel::WriteBlock => {
                    let (block_id, wlen) = with_ipc_buffer_mut(|ib| {
                        let regs = ib.msg_regs();
                        (regs[0] as _, regs[1] as _)
                    });
                    self.write_blocks(block_id, &buffer[..wlen]);
                    with_ipc_buffer_mut(|ib| {
                        sel4::reply(ib, rev_msg.build());
                    });
                }
            }
        }
    }
}
