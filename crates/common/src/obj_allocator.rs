use core::ops::Range;
use sel4::{cap::Untyped, init_thread::slot::CNODE};
use slot_manager::{LeafSlot, SlotManager};

pub struct ObjectAllocator {
    slot_manager: SlotManager,
    ut: Untyped,
}

impl ObjectAllocator {
    pub const fn empty() -> Self {
        Self {
            slot_manager: SlotManager::empty(),
            ut: sel4::cap::Untyped::from_bits(0),
        }
    }

    pub fn init(&mut self, empty_range: Range<usize>, untyped: Untyped) {
        self.slot_manager.init_empty_slots(empty_range);
        self.ut = untyped;
    }

    /// Allocate cap with Generic definition and size_bits before rebuilding the cspace
    pub fn allocate_variable_sized_origin<T: sel4::CapTypeForObjectOfVariableSize>(
        &mut self,
        size_bits: usize,
    ) -> sel4::Cap<T> {
        let leaf_slot = self.slot_manager.alloc_slot();
        self.ut
            .untyped_retype(
                &T::object_blueprint(size_bits),
                &sel4::init_thread::slot::CNODE
                    .cap()
                    .absolute_cptr_for_self(),
                leaf_slot.offset_of_cnode(),
                1,
            )
            .unwrap();
        leaf_slot.cap()
    }
}

impl ObjectAllocator {
    pub fn allocate_slot(&mut self) -> LeafSlot {
        let leaf_slot = self.slot_manager.alloc_slot();

        if leaf_slot.offset_of_cnode() == 0 {
            self.ut
                .untyped_retype(
                    &sel4::ObjectBlueprint::CNode { size_bits: 12 },
                    &sel4::init_thread::slot::CNODE
                        .cap()
                        .absolute_cptr_for_self(),
                    leaf_slot.offset_of_cnode(),
                    1,
                )
                .expect("can't allocate notification");
        }
        leaf_slot
    }

    /// Allocate the slot at the new cspace.
    pub fn allocate_and_retype(
        &mut self,
        blueprint: sel4::ObjectBlueprint,
    ) -> sel4::cap::Unspecified {
        let leaf_slot = self.allocate_slot();

        self.ut
            .untyped_retype(
                &blueprint,
                &leaf_slot.cnode_abs_cptr(),
                leaf_slot.offset_of_cnode(),
                1,
            )
            .unwrap();
        leaf_slot.cap()
    }

    /// Allocate the slot at the new cspace.
    pub fn retype_to_first(&mut self, blueprint: sel4::ObjectBlueprint) -> sel4::cap::Unspecified {
        self.ut
            .untyped_retype(
                &blueprint,
                &CNODE.cap().absolute_cptr_from_bits_with_depth(0, 52),
                0,
                1,
            )
            .unwrap();
        sel4::init_thread::Slot::from_index(0).cap()
    }

    /// Allocate and retype the slot at the new cspace
    pub fn allocate_and_retyped_fixed_sized<T: sel4::CapTypeForObjectOfFixedSize>(
        &mut self,
    ) -> sel4::Cap<T> {
        self.allocate_and_retype(T::object_blueprint()).cast()
    }

    /// ALlocate and retype the slot at the new cspace
    pub fn allocate_and_retyped_variable_sized<T: sel4::CapTypeForObjectOfVariableSize>(
        &mut self,
        size_bits: usize,
    ) -> sel4::Cap<T> {
        self.allocate_and_retype(T::object_blueprint(size_bits))
            .cast()
    }
}
