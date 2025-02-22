//! 串口模块初始化，查找串口服务
use common::services::{root::find_service, uart::UartService};
use spin::Once;

use crate::utils::obj::alloc_slot;

static UART_SERVICE: Once<UartService> = Once::new();

pub(super) fn init() {
    UART_SERVICE.call_once(|| {
        let slot = alloc_slot();
        find_service("uart-thread", slot).expect("can't find service");
        slot.into()
    });
}
