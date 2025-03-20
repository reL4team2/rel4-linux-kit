use alloc::vec::Vec;
use sel4::{
    CapTypeForObjectOfFixedSize,
    cap::{CNode, Endpoint, Granule, Notification, PT, Tcb, Untyped, VSpace},
    cap_type,
    init_thread::slot,
};
use slot_manager::LeafSlot;

pub struct ObjectAllocator {
    ut: Untyped,
}

impl ObjectAllocator {
    pub const fn empty() -> Self {
        Self {
            ut: sel4::cap::Untyped::from_bits(0),
        }
    }

    pub fn init(&mut self, untyped: Untyped) {
        self.ut = untyped;
    }

    /// Allocate cap with Generic definition and size_bits before rebuilding the cspace
    pub fn allocate_variable_sized_origin<T: sel4::CapTypeForObjectOfVariableSize>(
        &mut self,
        size_bits: usize,
    ) -> sel4::Cap<T> {
        let leaf_slot = super::slot::alloc_slot();
        self.ut
            .untyped_retype(
                &T::object_blueprint(size_bits),
                &slot::CNODE.cap().absolute_cptr_for_self(),
                leaf_slot.offset_of_cnode(),
                1,
            )
            .unwrap();
        leaf_slot.cap()
    }
}

impl ObjectAllocator {
    /// TODO: 申请多个位置，且判断位置是否超出
    pub fn allocate_slot(&mut self) -> LeafSlot {
        let leaf_slot = super::slot::alloc_slot();

        if leaf_slot.offset_of_cnode() == 0 {
            self.ut
                .untyped_retype(
                    &sel4::ObjectBlueprint::CNode { size_bits: 12 },
                    &sel4::init_thread::slot::CNODE
                        .cap()
                        .absolute_cptr_for_self(),
                    leaf_slot.cnode_idx(),
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
                &slot::CNODE.cap().absolute_cptr_from_bits_with_depth(0, 52),
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

    /// 申请一个物理页 [Granule]
    #[inline]
    pub fn alloc_page(&mut self) -> Granule {
        self.allocate_and_retype(cap_type::Granule::object_blueprint())
            .cast()
    }

    /// 申请一个 [Endpoint]
    #[inline]
    pub fn alloc_endpoint(&mut self) -> Endpoint {
        self.allocate_and_retype(cap_type::Endpoint::object_blueprint())
            .cast()
    }

    /// 申请一个 [CNode]
    #[inline]
    pub fn alloc_cnode(&mut self, size_bits: usize) -> CNode {
        self.allocate_and_retyped_variable_sized::<cap_type::CNode>(size_bits)
    }

    /// 申请一个 [VSpace]
    #[inline]
    pub fn alloc_vspace(&mut self) -> VSpace {
        self.allocate_and_retyped_fixed_sized::<cap_type::VSpace>()
    }

    /// 申请一个页表 [PT]
    #[inline]
    pub fn alloc_pt(&mut self) -> PT {
        self.allocate_and_retyped_fixed_sized::<cap_type::PT>()
    }

    /// 申请一个进程控制块 [Tcb]
    #[inline]
    pub fn alloc_tcb(&mut self) -> Tcb {
        self.allocate_and_retyped_fixed_sized::<cap_type::Tcb>()
    }

    /// 申请一个 Notification [Notification]
    pub fn alloc_notification(&mut self) -> Notification {
        self.allocate_and_retyped_fixed_sized::<cap_type::Notification>()
    }

    /// 申请多个页
    #[inline]
    pub fn alloc_pages(&mut self, pages: usize) -> Vec<Granule> {
        let leaf_slot = super::slot::alloc_slots(pages);

        self.ut
            .untyped_retype(
                &cap_type::Granule::object_blueprint(),
                &leaf_slot.cnode_abs_cptr(),
                leaf_slot.offset_of_cnode(),
                pages,
            )
            .unwrap();

        (0..pages)
            .map(|x| leaf_slot.next_nth_slot(x).cap())
            .collect()
    }
}
