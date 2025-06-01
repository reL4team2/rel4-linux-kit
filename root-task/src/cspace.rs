use common::config::CNODE_RADIX_BITS;
use sel4::{CNodeCapData, CapRights, cap::Null, cap_type, init_thread::slot};
use sel4_kit::slot_manager::LeafSlot;

use crate::OBJ_ALLOCATOR;

/// 重建 CSpace 空间
pub fn rebuild_cspace() {
    let cnode = OBJ_ALLOCATOR
        .lock()
        .allocate_variable_sized_origin::<cap_type::CNode>(CNODE_RADIX_BITS);
    cnode
        .absolute_cptr_from_bits_with_depth(0, CNODE_RADIX_BITS)
        .mint(
            &LeafSlot::from_slot(slot::CNODE).abs_cptr(),
            CapRights::all(),
            CNodeCapData::skip(0).into_word(),
        )
        .unwrap();
    // load
    LeafSlot::new(0)
        .abs_cptr()
        .mutate(
            &LeafSlot::from_slot(slot::CNODE).abs_cptr(),
            CNodeCapData::skip_high_bits(CNODE_RADIX_BITS).into_word(),
        )
        .unwrap();

    sel4::cap::CNode::from_bits(0)
        .absolute_cptr(slot::CNODE.cap())
        .mint(
            &sel4::cap::CNode::from_bits(0).absolute_cptr(cnode),
            CapRights::all(),
            CNodeCapData::skip_high_bits(CNODE_RADIX_BITS * 2).into_word(),
        )
        .unwrap();

    LeafSlot::new(0).delete().unwrap();

    slot::TCB
        .cap()
        .tcb_set_space(
            Null::from_bits(0).cptr(),
            cnode,
            CNodeCapData::skip_high_bits(2 * CNODE_RADIX_BITS),
            slot::VSPACE.cap(),
        )
        .unwrap();
}
