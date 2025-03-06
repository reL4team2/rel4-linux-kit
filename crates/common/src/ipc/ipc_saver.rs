//! 保存 IPC 的回复能力，以便之后回复
//!
//! 有些时候 IPC 并不能及时回复，需要满足一定条件后再回复，我们构建了一个 [IpcSaver] 来保存
//! 需要处理的 IPC

use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use sel4::{MessageInfo, cap::Endpoint};
use slot_manager::LeafSlot;

use crate::slot::alloc_slot;

/// 保存未能及时回复的 IPC
#[derive(Debug)]
pub struct IpcSaver {
    /// 等待队列
    queue: VecDeque<LeafSlot>,
    /// 闲置的 slot
    free_slots: Vec<LeafSlot>,
}

impl IpcSaver {
    /// 创建一个空的 [IpcSaver]
    pub const fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            free_slots: Vec::new(),
        }
    }

    /// 保存一个调用者的回复能力
    pub fn save_caller(&mut self) -> Result<(), sel4::Error> {
        let slot = match self.free_slots.pop() {
            Some(slot) => slot,
            None => alloc_slot(),
        };
        slot.save_caller()?;
        self.queue.push_back(slot);
        Ok(())
    }

    /// 回复一个 Endpoint
    ///
    /// - `msg`  [MessageInfo] 需要回复的消息
    pub fn reply_one(&mut self, msg: MessageInfo) -> Result<(), sel4::Error> {
        let reply_cap = self.queue.pop_front();

        if let Some(slot) = reply_cap {
            Endpoint::from(slot).send(msg);
            self.free_slots.push(slot);
        }
        Ok(())
    }

    /// 获取当前等待队列的长度
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }
}

impl Default for IpcSaver {
    fn default() -> Self {
        Self::new()
    }
}
