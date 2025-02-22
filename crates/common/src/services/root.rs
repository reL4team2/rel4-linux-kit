use common_macros::ipc_msg;
use crate_consts::DEFAULT_PARENT_EP;
use num_enum::{FromPrimitive, IntoPrimitive};
use sel4::{
    cap::{Endpoint, Null},
    init_thread, with_ipc_buffer, with_ipc_buffer_mut, MessageInfo, MessageInfoBuilder,
};
use slot_manager::LeafSlot;

use crate::services::IpcBufferRW;

#[ipc_msg]
#[derive(Debug, IntoPrimitive, FromPrimitive)]
#[repr(u64)]
pub enum RootEvent {
    Ping = 0x200,
    RegisterIRQ,
    TranslateAddr,
    FindService,
    AllocNotification,
    #[num_enum(catch_all)]
    Unknown(u64),
}

const ROOT_EP: Endpoint = Endpoint::from_bits(DEFAULT_PARENT_EP);

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

/// FIXME: This is not implemented
pub fn find_service(name: &str, target_slot: LeafSlot) -> Result<(), ()> {
    // let len = name.as_bytes().len();
    let mut off = 0;
    let origin_slot = with_ipc_buffer_mut(|ipc_buf| {
        ipc_buf.set_recv_slot(&target_slot.abs_cptr());
        name.write_buffer(ipc_buf, &mut off);
        // len.write_buffer(ipc_buf, &mut buf_idx);
        // name.write_buffer(ipc_buf, &mut buf_idx);
        // ipc_buf.msg_regs_mut()[0] = len as _;
        // ipc_buf.msg_bytes_mut()[REG_LEN..REG_LEN + len].copy_from_slice(name.as_bytes());

        // FIXME: using recv_slot()
        init_thread::slot::CNODE
            .cap()
            .absolute_cptr(Null::from_bits(0))
    });
    let msg = MessageInfoBuilder::default()
        .label(RootEvent::FindService.into())
        .length(off)
        .build();

    let msg = call(msg)?;
    assert_eq!(msg.extra_caps(), 1);
    with_ipc_buffer_mut(|ipc_buf| ipc_buf.set_recv_slot(&origin_slot));
    Ok(())
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

pub fn register_notify(target_slot: LeafSlot) -> Result<(), sel4::Error> {
    // construct the IPC message
    let origin_slot = with_ipc_buffer_mut(|ib| {
        ib.set_recv_slot(&target_slot.abs_cptr());

        // FIXME: using recv_slot()
        init_thread::slot::CNODE
            .cap()
            .absolute_cptr(Null::from_bits(0))
    });

    let msg = MessageInfoBuilder::default()
        .label(RootEvent::AllocNotification.into())
        .build();

    let recv_msg = call(msg).map_err(|_| sel4::Error::IllegalOperation)?;
    assert!(recv_msg.extra_caps() == 1);

    with_ipc_buffer_mut(|ib| ib.set_recv_slot(&origin_slot));

    Ok(())
}
