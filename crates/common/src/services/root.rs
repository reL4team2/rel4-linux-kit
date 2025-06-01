use common_macros::generate_ipc_send;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use sel4::{
    CapRights, MessageInfoBuilder,
    cap::{Endpoint, Null},
    init_thread, with_ipc_buffer, with_ipc_buffer_mut,
};
use sel4_kit::slot_manager::LeafSlot;

use crate::{
    config::{DEFAULT_PARENT_EP, REG_LEN},
    slot::alloc_slot,
};

#[derive(Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u64)]
pub enum RootEvent {
    Ping = 0x200,
    AllocNotification,
    AllocPage,
    FindService,
    RegisterIRQ,
    Shutdown,
    TranslateAddr,
    CreateChannel,
    JoinChannel,
}

const ROOT_EP: Endpoint = DEFAULT_PARENT_EP;

macro_rules! call_ep {
    ($msg:expr) => {
        ROOT_EP.call($msg)
    };
}

#[generate_ipc_send(label = RootEvent::Ping)]
pub fn ping() -> Result<(), ()> {
    call(MessageInfo::new(RootEvent::Ping.into(), 0, 0, 0))?;
    Ok(())
}

pub fn find_service(name: &str) -> Result<LeafSlot, sel4::Error> {
    with_ipc_buffer_mut(|ib| {
        let len = name.len();
        ib.msg_regs_mut()[0] = len as _;
        ib.msg_bytes_mut()[REG_LEN..][..len].copy_from_slice(name.as_bytes());
    });
    let msg = MessageInfoBuilder::default()
        .label(RootEvent::FindService.into())
        .length(1 + name.len().div_ceil(REG_LEN))
        .build();

    let msg = ROOT_EP.call(msg);
    if msg.extra_caps() == 0 {
        return Err(sel4::Error::FailedLookup);
    }
    let dst_slot = alloc_slot();
    LeafSlot::new(0).move_to(dst_slot)?;
    Ok(dst_slot)
}

#[generate_ipc_send(label = RootEvent::TranslateAddr)]
pub fn translate_addr(vaddr: usize) -> usize {}

pub fn register_irq(irq: usize, target_slot: LeafSlot) {
    // construct the IPC message
    let origin_slot = with_ipc_buffer_mut(|ipc_buffer| {
        ipc_buffer.set_recv_slot(&target_slot.abs_cptr());
        ipc_buffer.msg_regs_mut()[0] = irq as _;

        // FIXME: using recv_slot()
        init_thread::slot::CNODE
            .cap()
            .absolute_cptr(Null::from_bits(0))
    });

    let msg = MessageInfoBuilder::default()
        .label(RootEvent::RegisterIRQ.into())
        .length(1)
        .build();

    let recv_msg = ROOT_EP.call(msg);
    assert!(recv_msg.extra_caps() == 1);

    with_ipc_buffer_mut(|ipc_buffer| ipc_buffer.set_recv_slot(&origin_slot));
}

pub fn register_notify(target_slot: LeafSlot, badge: usize) -> Result<(), sel4::Error> {
    // construct the IPC message
    let recv_slot = LeafSlot::new(with_ipc_buffer(|ib| ib.recv_slot()).path().bits() as _);

    let msg = MessageInfoBuilder::default()
        .label(RootEvent::AllocNotification.into())
        .build();

    let recv_msg = ROOT_EP.call(msg);
    assert!(recv_msg.extra_caps() == 1);
    recv_slot.mint_to(target_slot, CapRights::all(), badge)?;
    recv_slot.delete()?;

    Ok(())
}

pub fn alloc_page(target_slot: LeafSlot, addr: usize) -> Result<LeafSlot, sel4::Error> {
    let recv_slot = with_ipc_buffer_mut(|ib| {
        ib.msg_regs_mut()[0] = addr as _;
        LeafSlot::new(ib.recv_slot().path().bits() as _)
    });

    let msg = MessageInfoBuilder::default()
        .length(1)
        .label(RootEvent::AllocPage.into())
        .build();

    let recv_msg = ROOT_EP.call(msg);
    assert!(recv_msg.extra_caps() == 1);
    recv_slot.move_to(target_slot)?;

    Ok(target_slot)
}

#[generate_ipc_send(label = RootEvent::CreateChannel)]
pub fn create_channel(addr: usize, page_count: usize) -> usize {}

#[generate_ipc_send(label = RootEvent::JoinChannel)]
pub fn join_channel(channel_id: usize, addr: usize) -> usize {}

/// 向 ROOT_EP 发送关机
#[generate_ipc_send(label = RootEvent::Shutdown)]
pub fn shutdown() -> ! {}
