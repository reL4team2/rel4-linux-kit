//! IPC Buffer
//!
//! 默认情况下 IPC_Buffer 在程序结尾后的一个页中
//!
//! 可以调用 [init_ipc_buffer] 初始化 IPC Buffer
//!
//! > Warning: 请注意 IPC Buffer 是 TLS 数据，请为每个线程都分配独占的一块内存
//!
use core::ptr;

use sel4::CapTypeForFrameObjectOfFixedSize;

/// 初始化 ipc_buffer
pub fn init_ipc_buffer() {
    sel4::set_ipc_buffer(get_ipc_buffer());
}

/// 获取 ipc_buffer 指针
///
/// 此指针在程序的结尾，ipc_buffer 是一块线程独占内存，暂时考虑为可变引用结构
/// NOTICE: 如果后续需要修改为其他结构也可以尝试正常工作
pub fn get_ipc_buffer() -> &'static mut sel4::IpcBuffer {
    unsafe {
        unsafe extern "C" {
            static _end: usize;
        }
        ((ptr::addr_of!(_end) as usize)
            .next_multiple_of(sel4::cap_type::Granule::FRAME_OBJECT_TYPE.bytes())
            as *mut sel4::IpcBuffer)
            .as_mut()
            .unwrap()
    }
}
