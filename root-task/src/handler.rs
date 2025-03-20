use core::sync::atomic::AtomicUsize;

use common::{
    page::PhysPage,
    services::{IpcBufferRW, root::RootEvent},
};
use config::PAGE_SIZE;
use sel4::{CapRights, Fault, IpcBuffer, MessageInfoBuilder, init_thread::slot, with_ipc_buffer};
use slot_manager::LeafSlot;

use crate::{OBJ_ALLOCATOR, RootTaskHandler};

impl RootTaskHandler {
    /// 处理 sel4 任务触发的异常，例如 VMFault
    pub fn handle_fault(&mut self, fault: Fault) {
        log::error!("[RootTask] Received {} Fault: {:#x?}", self.badge, fault);
        sel4_kit::arch::shutdown();
    }
    /// 等待任务传递的消息，并进行处理
    ///
    /// TODO: 使用统一的回复接口，使用统一的 Result，返回时返回 label 和 length。
    /// 如果有返回 Capability 如何处理？
    pub fn waiting_and_handle(&mut self, ib: &mut IpcBuffer) -> ! {
        let rev_msg = MessageInfoBuilder::default();
        let swap_slot = OBJ_ALLOCATOR.lock().allocate_slot();
        loop {
            let (message, badge) = self.fault_ep.recv(());
            self.badge = badge;
            let msg_label = RootEvent::from(message.label());

            match msg_label {
                RootEvent::Ping => sel4::reply(ib, rev_msg.build()),
                RootEvent::CreateChannel => {
                    static CHANNEL_ID: AtomicUsize = AtomicUsize::new(1);
                    let addr = ib.msg_regs()[0] as usize;
                    let page_count = ib.msg_regs()[1] as usize;
                    let pages = OBJ_ALLOCATOR.lock().alloc_pages(page_count);
                    pages
                        .iter()
                        .map(|x| {
                            let slot = OBJ_ALLOCATOR.lock().allocate_slot();
                            slot.copy_from(&LeafSlot::from_cap(*x), CapRights::all())
                                .unwrap();
                            slot
                        })
                        .enumerate()
                        .for_each(|(idx, x)| {
                            self.tasks[badge as usize]
                                .map_page(addr + idx * PAGE_SIZE, PhysPage::new(x.cap()));
                        });
                    let channel_id = CHANNEL_ID.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
                    self.channels.push((channel_id, pages));
                    ib.msg_regs_mut()[0] = channel_id as u64;
                    sel4::reply(ib, rev_msg.length(1).build());
                }
                RootEvent::JoinChannel => {
                    let channel_id = ib.msg_regs()[0] as usize;
                    let addr = ib.msg_regs()[1] as usize;
                    if let Some((_, pages)) = self.channels.iter().find(|x| x.0 == channel_id) {
                        pages
                            .iter()
                            .map(|x| {
                                let slot = OBJ_ALLOCATOR.lock().allocate_slot();
                                slot.copy_from(&LeafSlot::from_cap(*x), CapRights::all())
                                    .unwrap();
                                slot
                            })
                            .enumerate()
                            .for_each(|(idx, x)| {
                                self.tasks[badge as usize]
                                    .map_page(addr + idx * PAGE_SIZE, PhysPage::new(x.cap()));
                            });
                        ib.msg_regs_mut()[0] = (pages.len() * PAGE_SIZE) as u64;
                        sel4::reply(ib, rev_msg.length(1).build());
                    }
                }
                RootEvent::TranslateAddr => {
                    let mut off = 0;
                    let addr = usize::read_buffer(ib, &mut off);

                    let phys_addr = self.tasks[badge as usize]
                        .mapped_page
                        .get(&(addr & !0xfff))
                        .map(|x| x.addr())
                        .unwrap();

                    ib.msg_regs_mut()[0] = (phys_addr + addr % 0x1000) as _;
                    sel4::reply(ib, rev_msg.length(off).build());
                }
                RootEvent::FindService => {
                    let name = <&str>::read_buffer(ib, &mut 0);
                    let task = self.tasks.iter().find(|task| task.name == name);
                    let msg = match task {
                        Some(task) => {
                            LeafSlot::from(task.srv_ep)
                                .mint_to(swap_slot, CapRights::all(), badge as _)
                                .unwrap();
                            ib.caps_or_badges_mut()[0] = swap_slot.raw() as _;
                            rev_msg.extra_caps(1).build()
                        }
                        // 发生错误时返回值 不为 -1
                        None => rev_msg.label(1).build(),
                    };
                    sel4::reply(ib, msg);
                    let _ = swap_slot.delete();
                }
                // Allocate a irq handler capability
                // Transfer it to the requested service
                RootEvent::RegisterIRQ => {
                    let irq = ib.msg_regs()[0];
                    let dst_slot = LeafSlot::new(0);
                    slot::IRQ_CONTROL
                        .cap()
                        .irq_control_get(irq, &dst_slot.abs_cptr())
                        .unwrap();

                    ib.caps_or_badges_mut()[0] = 0;
                    sel4::reply(ib, rev_msg.extra_caps(1).build());

                    dst_slot.delete().unwrap();
                }
                // 申请一个 Notification Capability
                RootEvent::AllocNotification => {
                    // 在 0 的 slot 处创建一个 Capability
                    OBJ_ALLOCATOR
                        .lock()
                        .retype_to_first(sel4::ObjectBlueprint::Notification);

                    ib.caps_or_badges_mut()[0] = 0;
                    sel4::reply(ib, rev_msg.extra_caps(1).build());

                    LeafSlot::new(0).delete().unwrap();
                }
                RootEvent::AllocPage => {
                    assert_eq!(message.length(), 1);
                    let addr = ib.msg_regs()[0] as usize;
                    let page = OBJ_ALLOCATOR.lock().alloc_page();
                    self.tasks[badge as usize].map_page(addr, PhysPage::new(page));
                    LeafSlot::new(0)
                        .copy_from(&LeafSlot::new(page.bits() as _), CapRights::all())
                        .unwrap();
                    ib.caps_or_badges_mut()[0] = 0;
                    sel4::reply(ib, rev_msg.extra_caps(1).build());
                    LeafSlot::new(0).delete().unwrap();
                }
                RootEvent::Shutdown => sel4_kit::arch::shutdown(),
                RootEvent::Unknown(label) => {
                    if label >= 8 {
                        log::error!("Unknown root messaage label: {label}")
                    }
                    let fault = with_ipc_buffer(|buffer| Fault::new(buffer, &message));
                    self.handle_fault(fault);
                }
            }
        }
    }
}
