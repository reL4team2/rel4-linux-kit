use common_macros::generate_ipc_send;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use sel4::cap::Endpoint;
use slot_manager::LeafSlot;
use zerocopy::IntoBytes;

#[derive(Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u64)]
pub enum UartEvent {
    Ping,
    GetChar,
    PutChar,
    PutString,
}

pub struct UartService {
    ep: Endpoint,
}

impl UartService {
    pub const fn from_bits(bits: u64) -> Self {
        Self::new(Endpoint::from_bits(bits))
    }

    pub const fn from_leaf_slot(ls: LeafSlot) -> Self {
        Self::from_bits(ls.raw() as _)
    }

    pub const fn leaf_slot(&self) -> LeafSlot {
        LeafSlot::new(self.ep.bits() as _)
    }

    pub const fn new(endpoint: Endpoint) -> Self {
        Self { ep: endpoint }
    }
}

impl UartService {
    #[generate_ipc_send(label = UartEvent::PutChar)]
    pub fn send(&self, c: u8) {}

    #[generate_ipc_send(label = UartEvent::GetChar)]
    pub fn getchar(&self) -> u8 {}

    #[generate_ipc_send(label = UartEvent::Ping)]
    pub fn ping(&self) {}

    #[generate_ipc_send(label = UartEvent::PutString)]
    pub fn puts(&self, s: &[u8]) {}
}

impl From<LeafSlot> for UartService {
    fn from(value: LeafSlot) -> Self {
        Self::from_leaf_slot(value)
    }
}
