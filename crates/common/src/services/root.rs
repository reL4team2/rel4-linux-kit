use common_macros::ipc_msg;
use num_enum::{FromPrimitive, IntoPrimitive};
use sel4::{
    CapRights, MessageInfo, MessageInfoBuilder,
    cap::{Endpoint, Null},
    init_thread, with_ipc_buffer, with_ipc_buffer_mut,
};
use slot_manager::LeafSlot;

use crate::{consts::DEFAULT_PARENT_EP, services::IpcBufferRW, slot::alloc_slot};

#[ipc_msg]
#[derive(Debug, IntoPrimitive, FromPrimitive)]
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
    #[num_enum(catch_all)]
    Unknown(u64),
}

const ROOT_EP: Endpoint = DEFAULT_PARENT_EP;

fn call(msg: MessageInfo) -> Result<MessageInfo, ()> {
    let msg = ROOT_EP.call(msg);
    if msg.label() != 0 {
        return Err(());
    }
    Ok(msg)
}

pub fn ping() -> Result<(), ()> {
    call(MessageInfo::new(RootEvent::Ping.into(), 0, 0, 0))?;
    Ok(())
}

pub fn find_service(name: &str) -> Result<LeafSlot, sel4::Error> {
    let mut off = 0;
    with_ipc_buffer_mut(|ipc_buf| name.write_buffer(ipc_buf, &mut off));
    let msg = MessageInfoBuilder::default()
        .label(RootEvent::FindService.into())
        .length(off)
        .build();

    let msg = call(msg).map_err(|_| sel4::Error::IllegalOperation)?;
    if msg.extra_caps() == 0 {
        return Err(sel4::Error::FailedLookup);
    }
    let dst_slot = alloc_slot();
    LeafSlot::new(0).move_to(dst_slot)?;
    Ok(dst_slot)
}

pub fn translate_addr(vaddr: usize) -> Result<usize, ()> {
    // construct the ipc message
    with_ipc_buffer_mut(|ipc_buf| ipc_buf.msg_regs_mut()[0] = vaddr as _);

    // Send a ipc message
    let msg = MessageInfoBuilder::default()
        .label(RootEvent::TranslateAddr.into())
        .length(1)
        .build();
    call(msg)?;

    // Get the physical address
    let paddr = with_ipc_buffer(|ipc_buffer| ipc_buffer.msg_regs()[0]);
    Ok(paddr as _)
}

pub fn register_irq(irq: usize, target_slot: LeafSlot) -> Result<(), sel4::Error> {
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

    let recv_msg = call(msg).map_err(|_| sel4::Error::IllegalOperation)?;
    assert!(recv_msg.extra_caps() == 1);

    with_ipc_buffer_mut(|ipc_buffer| ipc_buffer.set_recv_slot(&origin_slot));

    Ok(())
}

pub fn register_notify(target_slot: LeafSlot, badge: usize) -> Result<(), sel4::Error> {
    // construct the IPC message
    let recv_slot = LeafSlot::new(with_ipc_buffer(|ib| ib.recv_slot()).path().bits() as _);

    let msg = MessageInfoBuilder::default()
        .label(RootEvent::AllocNotification.into())
        .build();

    let recv_msg = call(msg).map_err(|_| sel4::Error::IllegalOperation)?;
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

    let recv_msg = call(msg).map_err(|_| sel4::Error::IllegalOperation)?;
    assert!(recv_msg.extra_caps() == 1);
    recv_slot.move_to(target_slot)?;

    Ok(target_slot)
}

pub fn create_channel(addr: usize, page_count: usize) -> Result<usize, sel4::Error> {
    with_ipc_buffer_mut(|ib| {
        ib.msg_regs_mut()[0] = addr as u64;
        ib.msg_regs_mut()[1] = page_count as u64;
    });

    let msg = MessageInfoBuilder::default()
        .length(2)
        .label(RootEvent::CreateChannel.into())
        .build();

    let ret = call(msg).map_err(|_| sel4::Error::IllegalOperation)?;
    assert_eq!(ret.label(), 0);
    with_ipc_buffer(|ib| Ok(ib.msg_regs()[0] as _))
}

pub fn join_channel(channel_id: usize, addr: usize) -> Result<usize, sel4::Error> {
    with_ipc_buffer_mut(|ib| {
        ib.msg_regs_mut()[0] = channel_id as u64;
        ib.msg_regs_mut()[1] = addr as u64;
    });

    let msg = MessageInfoBuilder::default()
        .length(2)
        .label(RootEvent::JoinChannel.into())
        .build();

    let ret = call(msg).map_err(|_| sel4::Error::IllegalOperation)?;
    assert_eq!(ret.label(), 0);
    with_ipc_buffer(|ib| Ok(ib.msg_regs()[0] as _))
}

/// 向 ROOT_EP 发送关机
pub fn shutdown() -> Result<(), sel4::Error> {
    call(
        MessageInfoBuilder::default()
            .label(RootEvent::Shutdown.into())
            .build(),
    )
    .map_err(|_| sel4::Error::IllegalOperation)?;
    Ok(())
}
