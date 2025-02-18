use common_macros::ipc_msg;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use sel4::{
    cap::{Endpoint, Null},
    init_thread, with_ipc_buffer, with_ipc_buffer_mut, AbsoluteCPtr, MessageInfo,
    MessageInfoBuilder,
};
use slot_manager::LeafSlot;

use crate::services::IpcBufferRW;

#[ipc_msg]
#[derive(Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u64)]
pub enum RootMessageLabel {
    Ping = 0x200,
    RegisterIRQ,
    TranslateAddr,
    FindService,
    AllocNotification,
}

pub struct RootService(Endpoint);

impl RootService {
    pub const fn from_bits(bits: u64) -> Self {
        Self::new(Endpoint::from_bits(bits))
    }

    pub const fn new(endpoint: Endpoint) -> Self {
        Self(endpoint)
    }

    pub fn call(&self, msg: MessageInfo) -> Result<MessageInfo, ()> {
        let msg = self.0.call(msg);
        if msg.label() != 0 {
            return Err(());
        }
        Ok(msg)
    }

    pub fn ping(&self) -> Result<(), ()> {
        self.call(MessageInfo::new(RootMessageLabel::Ping.into(), 0, 0, 0))?;
        Ok(())
    }

    /// FIXME: This is not implemented
    pub fn find_service(&self, name: &str, target_slot: LeafSlot) -> Result<(), ()> {
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
            .label(RootMessageLabel::FindService.into())
            .length(off)
            .build();

        let msg = self.call(msg)?;
        assert_eq!(msg.extra_caps(), 1);
        with_ipc_buffer_mut(|ipc_buf| ipc_buf.set_recv_slot(&origin_slot));
        Ok(())
    }

    pub fn translate_addr(&self, vaddr: usize) -> Result<usize, ()> {
        // construct the ipc message
        with_ipc_buffer_mut(|ipc_buf| ipc_buf.msg_regs_mut()[0] = vaddr as _);

        // Send a ipc message
        let msg = MessageInfoBuilder::default()
            .label(RootMessageLabel::TranslateAddr.into())
            .length(1)
            .build();
        self.call(msg)?;

        // Get the physical address
        let paddr = with_ipc_buffer(|ipc_buffer| ipc_buffer.msg_regs()[0]);
        Ok(paddr as _)
    }

    pub fn register_irq(&self, irq: usize, target_slot: AbsoluteCPtr) -> Result<(), sel4::Error> {
        // construct the IPC message
        let origin_slot = with_ipc_buffer_mut(|ipc_buffer| {
            ipc_buffer.set_recv_slot(&target_slot);
            ipc_buffer.msg_regs_mut()[0] = irq as _;

            // FIXME: using recv_slot()
            init_thread::slot::CNODE
                .cap()
                .absolute_cptr(Null::from_bits(0))
        });

        let msg = MessageInfoBuilder::default()
            .label(RootMessageLabel::RegisterIRQ.into())
            .length(1)
            .build();

        let recv_msg = self.call(msg).map_err(|_| sel4::Error::IllegalOperation)?;
        assert!(recv_msg.extra_caps() == 1);

        with_ipc_buffer_mut(|ipc_buffer| ipc_buffer.set_recv_slot(&origin_slot));

        Ok(())
    }

    pub fn alloc_notification(&self, target_slot: AbsoluteCPtr) -> Result<(), sel4::Error> {
        // construct the IPC message
        let origin_slot = with_ipc_buffer_mut(|ib| {
            ib.set_recv_slot(&target_slot);

            // FIXME: using recv_slot()
            init_thread::slot::CNODE
                .cap()
                .absolute_cptr(Null::from_bits(0))
        });

        let msg = MessageInfoBuilder::default()
            .label(RootMessageLabel::AllocNotification.into())
            .build();

        let recv_msg = self.call(msg).map_err(|_| sel4::Error::IllegalOperation)?;
        assert!(recv_msg.extra_caps() == 1);

        with_ipc_buffer_mut(|ib| ib.set_recv_slot(&origin_slot));

        Ok(())
    }
}
