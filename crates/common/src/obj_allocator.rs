use sel4::{
    Cap, CapTypeForObjectOfFixedSize,
    cap::{CNode, Endpoint, Granule, Notification, PT, Tcb, Untyped, VSpace, LargePage},
    cap_type::{self},
    init_thread::slot,
};
use sel4_kit::slot_manager::LeafSlot;
use spin::Mutex;

pub struct ObjectAllocator {
    ut: Mutex<Untyped>,
}

impl ObjectAllocator {
    pub const fn empty() -> Self {
        Self {
            ut: Mutex::new(sel4::cap::Untyped::from_bits(0)),
        }
    }

    pub fn init(&self, untyped: Untyped) {
        *self.ut.lock() = untyped;
    }

    fn untyped(&self) -> Untyped {
        *self.ut.lock()
    }

    /// Allocate cap with Generic definition and size_bits before rebuilding the cspace
    pub fn allocate_variable_sized_origin<T: sel4::CapTypeForObjectOfVariableSize>(
        &self,
        size_bits: usize,
    ) -> sel4::Cap<T> {
        let leaf_slot = super::slot::alloc_slot();
        self.untyped()
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
    #[inline]
    pub fn alloc_untyped(&self, size_bits: usize) -> Cap<cap_type::Untyped> {
        self.allocate_and_retyped_variable_sized(size_bits)
    }

    /// TODO: 申请多个位置，且判断位置是否超出
    pub fn allocate_slot(&self) -> LeafSlot {
        super::slot::alloc_slot()
    }

    pub fn extend_slot(&self, slot: LeafSlot) {
        loop {
            let cap_alloc = self.untyped().untyped_retype(
                &sel4::ObjectBlueprint::CNode { size_bits: 12 },
                &sel4::init_thread::slot::CNODE
                    .cap()
                    .absolute_cptr_for_self(),
                slot.cnode_idx(),
                1,
            );
            if cap_alloc == Ok(()) {
                break;
            } else if cap_alloc == Err(sel4::Error::NotEnoughMemory) {
                LeafSlot::from_cap(self.untyped()).delete().unwrap();
                crate::root::alloc_untyped(self.untyped().into()).unwrap();
            } else {
                cap_alloc.unwrap();
            }
        }
    }

    /// Allocate the slot at the new cspace.
    pub fn allocate_and_retype(&self, blueprint: sel4::ObjectBlueprint) -> sel4::cap::Unspecified {
        let leaf_slot = self.allocate_slot();
        loop {
            let cap_alloc = self.untyped().untyped_retype(
                &blueprint,
                &leaf_slot.cnode_abs_cptr(),
                leaf_slot.offset_of_cnode(),
                1,
            );
            if cap_alloc == Ok(()) {
                break;
            } else if cap_alloc == Err(sel4::Error::NotEnoughMemory) {
                LeafSlot::from_cap(self.untyped()).delete().unwrap();
                crate::root::alloc_untyped(self.untyped().into()).unwrap();
            } else {
                cap_alloc.unwrap();
            }
        }
        leaf_slot.cap()
    }

    /// Allocate the slot at the new cspace.
    pub fn retype_to_first(&self, blueprint: sel4::ObjectBlueprint) -> sel4::cap::Unspecified {
        self.untyped()
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
        &self,
    ) -> sel4::Cap<T> {
        self.allocate_and_retype(T::object_blueprint()).cast()
    }

    /// ALlocate and retype the slot at the new cspace
    pub fn allocate_and_retyped_variable_sized<T: sel4::CapTypeForObjectOfVariableSize>(
        &self,
        size_bits: usize,
    ) -> sel4::Cap<T> {
        self.allocate_and_retype(T::object_blueprint(size_bits))
            .cast()
    }

    /// 申请一个物理页 [Granule]
    #[inline]
    pub fn alloc_page(&self) -> Granule {
        self.allocate_and_retype(cap_type::Granule::object_blueprint())
            .cast()
    }

    /// 申请一个 [Endpoint]
    #[inline]
    pub fn alloc_endpoint(&self) -> Endpoint {
        self.allocate_and_retype(cap_type::Endpoint::object_blueprint())
            .cast()
    }

    /// 申请一个 [CNode]
    #[inline]
    pub fn alloc_cnode(&self, size_bits: usize) -> CNode {
        self.allocate_and_retyped_variable_sized::<cap_type::CNode>(size_bits)
    }

    /// 申请一个 [VSpace]
    #[inline]
    pub fn alloc_vspace(&self) -> VSpace {
        self.allocate_and_retyped_fixed_sized::<cap_type::VSpace>()
    }

    /// 申请一个页表 [PT]
    #[inline]
    pub fn alloc_pt(&self) -> PT {
        self.allocate_and_retyped_fixed_sized::<cap_type::PT>()
    }

    /// 申请一个进程控制块 [Tcb]
    #[inline]
    pub fn alloc_tcb(&self) -> Tcb {
        self.allocate_and_retyped_fixed_sized::<cap_type::Tcb>()
    }

    /// 申请一个 Notification [Notification]
    pub fn alloc_notification(&self) -> Notification {
        self.allocate_and_retyped_fixed_sized::<cap_type::Notification>()
    }

    /// 申请一个大页
    pub fn alloc_large_page(&self) -> LargePage {
        self.allocate_and_retype(cap_type::LargePage::object_blueprint())
            .cast()
    }

    /// 申请多个页
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn alloc_pages(&self, pages: usize) -> alloc::vec::Vec<Granule> {
        let leaf_slot = super::slot::alloc_slots(pages);

        self.untyped()
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

    /// 申请多个大页
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn alloc_large_pages(&self, pages: usize) -> alloc::vec::Vec<LargePage> {
        let leaf_slot = super::slot::alloc_slots(pages);

        self.untyped()
            .untyped_retype(
                &cap_type::LargePage::object_blueprint(),
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
