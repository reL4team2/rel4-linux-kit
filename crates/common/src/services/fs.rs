use common_macros::ipc_msg;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use sel4::{cap::Endpoint, MessageInfo, MessageInfoBuilder};
use slot_manager::LeafSlot;

#[derive(Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u64)]
pub enum FileServiceLabel {
    Ping,
    ReadDir,
}

pub struct FileSerivce {
    ep_cap: Endpoint,
}

impl FileSerivce {
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
            .label(FileServiceLabel::Ping.into())
            .build();
        self.call(ping_msg)
    }

    // FIXME: 应该返回一个数组或者一个结构表示所有的文件
    // 功能类似于 getdents
    pub fn read_dir(&self, _dir: &str) -> Result<(), ()> {
        let msg = MessageInfoBuilder::default()
            .label(FileServiceLabel::ReadDir.into())
            .build();
        let msg = self.call(msg)?;
        Ok(())
    }
}
