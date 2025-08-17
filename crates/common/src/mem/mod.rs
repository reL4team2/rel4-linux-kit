//! seL4 Cap 内存管理模块
//!
//!

use alloc::vec::Vec;
use sel4::{
    Cap, CapTypeForObjectOfFixedSize, CapTypeForObjectOfVariableSize, cap::Untyped, cap_type,
};
use sel4_kit::slot_manager::LeafSlot;

use crate::slot::alloc_slot;

pub struct CapMemSet {
    /// (Untyped, size in bytes)
    untypes: Vec<(Untyped, usize)>,
    alloc_func: Option<fn() -> (Untyped, usize)>,
}

impl CapMemSet {
    pub fn new(alloc_func: Option<fn() -> (Untyped, usize)>) -> Self {
        CapMemSet {
            untypes: Vec::new(),
            alloc_func,
        }
    }

    pub fn check_available(&mut self, size: usize) {
        if let Some((_, available)) = self.untypes.last() {
            if *available > size {
                return;
            }
        }
        if let Some(func) = self.alloc_func {
            let (untyped, available) = func();
            if available >= size {
                self.untypes.push((untyped, available));
                return;
            }
        }
        panic!(
            "No available untyped memory for allocation of size {}",
            size
        );
    }

    pub fn untyped_list(&self) -> &[(Untyped, usize)] {
        &self.untypes
    }

    pub fn add(&mut self, untyped: Untyped, size: usize) {
        self.untypes.push((untyped, size));
    }

    pub fn alloc_fixed<T: CapTypeForObjectOfFixedSize>(&mut self) -> LeafSlot {
        let dst = alloc_slot();
        let phys_size = 1 << T::object_blueprint().physical_size_bits();
        self.check_available(phys_size);
        let last = self
            .untypes
            .last_mut()
            .expect("No untyped memory available");
        last.1 -= phys_size;
        last.0
            .untyped_retype(
                &T::object_blueprint(),
                &dst.cnode_abs_cptr(),
                dst.offset_of_cnode(),
                1,
            )
            .unwrap();
        dst
    }

    pub fn alloc_variable<T: CapTypeForObjectOfVariableSize>(
        &mut self,
        size_bits: usize,
    ) -> LeafSlot {
        let dst = alloc_slot();
        let phys_size = 1 << T::object_blueprint(size_bits).physical_size_bits();
        self.check_available(phys_size);
        let last = self
            .untypes
            .last_mut()
            .expect("No untyped memory available");
        last.1 -= phys_size;
        last.0
            .untyped_retype(
                &T::object_blueprint(size_bits),
                &dst.cnode_abs_cptr(),
                dst.offset_of_cnode(),
                1,
            )
            .unwrap();
        dst
    }

    #[inline]
    pub fn alloc_page(&mut self) -> Cap<cap_type::Granule> {
        self.alloc_fixed::<cap_type::Granule>().into()
    }

    #[inline]
    pub fn alloc_pt(&mut self) -> Cap<cap_type::PT> {
        self.alloc_fixed::<cap_type::PT>().into()
    }

    #[inline]
    pub fn alloc_vspace(&mut self) -> Cap<cap_type::VSpace> {
        self.alloc_fixed::<cap_type::VSpace>().into()
    }

    #[inline]
    pub fn alloc_tcb(&mut self) -> Cap<cap_type::Tcb> {
        self.alloc_fixed::<cap_type::Tcb>().into()
    }

    /// 申请一个 [CNode]
    #[inline]
    pub fn alloc_cnode(&mut self, size_bits: usize) -> Cap<cap_type::CNode> {
        self.alloc_variable::<cap_type::CNode>(size_bits).into()
    }

    #[inline]
    pub fn alloc_notification(&mut self) -> Cap<cap_type::Notification> {
        self.alloc_fixed::<cap_type::Notification>().into()
    }
}
