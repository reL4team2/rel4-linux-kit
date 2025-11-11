//! 为 seL4 用户态程序提供中断支持

use crate::config::PPI_NUM;
use common::{ObjectAllocator, root::register_irq, slot::alloc_slot};
use core::sync::atomic::{AtomicUsize, Ordering};
use sel4::cap::Notification;
use sel4_kit::slot_manager::LeafSlot;

/// 中断管理模块
/// 提供中断注册和注销功能
/// 这是一个 lock-free 的实现
pub struct IrqManager<'a, const N: usize> {
    global_notify: Notification,
    cpu_id: usize,
    obj_allocator: &'a ObjectAllocator,
    irq_caps: [AtomicUsize; N],
}

impl<'a, const N: usize> IrqManager<'a, N> {
    /// 创建一个新的中断管理器实例
    pub fn new(cpu_id: usize, obj_allocator: &'a ObjectAllocator) -> Self {
        Self {
            global_notify: Notification::from_bits(0),
            cpu_id,
            obj_allocator,
            irq_caps: [const { AtomicUsize::new(0) }; N],
        }
    }

    /// 初始化中断管理器
    pub fn init(&mut self, _cpu_id: usize) -> sel4::Result<()> {
        self.global_notify = self.obj_allocator.alloc_notification();
        sel4::init_thread::slot::TCB
            .cap()
            .tcb_bind_notification(self.global_notify)?;
        Ok(())
    }

    /// 注册一个中断号，并返回对应的通知对象
    pub fn register_irq(&self, irq: usize) -> sel4::Result<bool> {
        let idx = self.cpu_id * PPI_NUM + irq;
        let notify_slot = alloc_slot();
        LeafSlot::from_cap(self.global_notify).mint_to(
            notify_slot,
            sel4::CapRights::all(),
            irq as _,
        )?;

        let irq_slot = alloc_slot();
        register_irq(idx as _, irq_slot);

        irq_slot
            .cap()
            .irq_handler_set_notification(notify_slot.cap())?;
        irq_slot.cap().irq_handler_ack()?;

        // 将 irq slot 的序号存到原子变量中
        Ok(self.irq_caps[idx]
            .compare_exchange(0, irq_slot.raw(), Ordering::Acquire, Ordering::Relaxed)
            .is_ok())
    }

    /// 注销一个中断号，释放对应的通知对象
    pub fn unregister_irq(&self, _irq: usize, _notify: Notification) -> sel4::Result<()> {
        // TODO: 卸载中断号的实现
        Ok(())
    }

    /// Ack 一个中断
    pub fn ack_irq(&self, idx: usize) {
        let irq_idx = self.irq_caps[idx].load(Ordering::Acquire);
        let irq_slot = LeafSlot::new(irq_idx);
        irq_slot.cap().irq_handler_ack().unwrap();
    }
}
