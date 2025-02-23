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

/// 从 [UartService] 中读取一个字符 (u8)
///
/// 如果没有读取到任何的数，直接返回 [Option::None]
#[inline]
pub fn get_char() -> Option<u8> {
    UART_SERVICE.get().unwrap().getchar().ok()
}
