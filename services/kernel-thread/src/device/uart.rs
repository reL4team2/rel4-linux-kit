//! 串口模块初始化，查找串口服务
use srv_gate::UART_IMPLS;

pub(super) fn init() {
    UART_IMPLS[0].lock().init();
}

/// 从 [UartService] 中读取一个字符 (u8)
///
/// 如果没有读取到任何的数，直接返回 [Option::None]
#[inline]
pub fn get_char() -> Option<u8> {
    Some(UART_IMPLS[0].lock().getchar())
}
