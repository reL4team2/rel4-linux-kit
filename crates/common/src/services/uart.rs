use num_enum::{IntoPrimitive, TryFromPrimitive};
use sel4::{MessageInfo, MessageInfoBuilder, cap::Endpoint, with_ipc_buffer};
use slot_manager::LeafSlot;

#[derive(Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u64)]
pub enum UartEvent {
    Ping,
    GetChar,
}

pub struct UartService {
    ep_cap: Endpoint,
}

impl UartService {
    pub const fn from_bits(bits: u64) -> Self {
        Self::new(Endpoint::from_bits(bits))
    }

    pub const fn from_leaf_slot(ls: LeafSlot) -> Self {
        Self::from_bits(ls.raw() as _)
    }

    pub const fn leaf_slot(&self) -> LeafSlot {
        LeafSlot::new(self.ep_cap.bits() as _)
    }

    pub const fn new(endpoint: Endpoint) -> Self {
        Self { ep_cap: endpoint }
    }

    pub fn call(&self, msg: MessageInfo) -> Result<MessageInfo, ()> {
        let msg = self.ep_cap.call(msg);
        if msg.label() != 0 {
            return Err(());
        }
        Ok(msg)
    }

    pub fn ping(&self) -> Result<MessageInfo, ()> {
        let ping_msg = MessageInfoBuilder::default()
            .label(UartEvent::Ping.into())
            .build();
        self.call(ping_msg)
    }

    pub fn getchar(&self) -> Result<u8, ()> {
        let message = MessageInfoBuilder::default()
            .label(UartEvent::GetChar.into())
            .build();
        let msg = self.call(message)?;
        assert_ne!(msg.length(), 0);
        with_ipc_buffer(|ipc_buffer| Ok(ipc_buffer.msg_bytes()[0]))
    }
}

impl From<LeafSlot> for UartService {
    fn from(value: LeafSlot) -> Self {
        Self::from_leaf_slot(value)
    }
}
