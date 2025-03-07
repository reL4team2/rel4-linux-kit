use crate_consts::DEFAULT_PARENT_EP;
use sel4::{
    CNodeCapData, UserContext,
    cap::{SmallPage, Tcb},
    init_thread::slot,
};

pub fn create_thread(
    tcb: Tcb,
    mut ctx: UserContext,
    ipc_addr: usize,
    ipc_cap: SmallPage,
) -> Result<(), sel4::Error> {
    tcb.tcb_configure(
        DEFAULT_PARENT_EP.cptr(),
        slot::CNODE.cap(),
        CNodeCapData::new(0, 0),
        slot::VSPACE.cap(),
        ipc_addr as _,
        ipc_cap,
    )?;
    tcb.tcb_set_sched_params(slot::TCB.cap(), 0, 255)?;
    tcb.tcb_write_all_registers(true, &mut ctx)
}
