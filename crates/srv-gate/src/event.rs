use common::config::DEFAULT_SERVE_EP;
use linkme::distributed_slice;
use sel4::MessageInfo;

pub struct EventHandler {
    pub event: u64,
    pub callback: fn(&MessageInfo, u64),
}

#[distributed_slice]
pub static EVENT_HANDLERS: [EventHandler];

/// 定义一个事件处理器，利用 paste! 将 中断处理号加入到名称中用于做标识，防止 irq 冲突
#[macro_export]
macro_rules! def_event_handler {
    ($name:ident, $event:expr, $handler:expr) => {
        $crate::paste! {
            #[$crate::linkme::distributed_slice($crate::event::EVENT_HANDLERS)]
            #[linkme(crate = $crate::linkme)]
            #[unsafe(no_mangle)]
            pub static $name: $crate::event::EventHandler = $crate::event::EventHandler {
                event: $event as _,
                callback: $handler
            };
        }
    };
}

pub fn handle_events() {
    let (ref mut msg, badge) = DEFAULT_SERVE_EP.recv(());
    let event = msg.label();
    EVENT_HANDLERS
        .iter()
        .find(|x| x.event == event)
        .inspect(|EventHandler { callback, .. }| callback(msg, badge));
    log::error!("Not found any callback for event: {:#x}", event);
}
