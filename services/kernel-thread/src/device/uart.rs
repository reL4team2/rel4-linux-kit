use common::services::uart::UartService;
use spin::Once;

use crate::utils::service::find_service;

static UART_SERVICE: Once<UartService> = Once::new();

pub(super) fn init() {
    UART_SERVICE.call_once(|| {
        find_service("uart-thread")
            .expect("can't find service")
            .into()
    });
}
