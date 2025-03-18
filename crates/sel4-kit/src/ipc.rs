//! 提供 ipc 相关支持
//!
//! 提供异步的 [Notification] `poll`

use core::task::Poll;

use sel4::{
    MessageInfo,
    cap::{Endpoint, Notification},
    with_ipc_buffer_mut,
};

///// poll 一个 [Notification]
/////
///// 这是一个异步操作，当 poll 的时候会检查 [Notification] 是否有 signal，如果有则返回相关的 badge，
///// 如果没有，则返回 0 作为 badge
/////
///// **TIPS: 如果创建一个默认的 [Notification] 之后没有设置的话，那么处于 unbadged 状体，即便有也是 None**
//pub fn poll_notification(noti: Notification) -> Option<u64> {
//    let (_, badge) = with_ipc_buffer_mut(|ib| ib.inner_mut().seL4_Poll(noti.bits()));
//    match badge {
//        0 => Option::None,
//        _ => Option::Some(badge),
//    }
//}
//
///// poll 一个 [Endpoint]
/////
///// 这是一个异步操作，当 poll 的时候会检查 [Endpoint] 是否有消息，如果有则返回相关的 消息和badge，
///// 如果没有，则返回 0 作为 badge
/////
///// **TIPS: 如果创建一个默认的 [Endpoint] 之后没有设置的话，那么处于 unbadged 状体，即便有也是 None**
//pub fn poll_endpoint(ep: Endpoint) -> Option<(MessageInfo, u64)> {
//    let (msg, badge) = with_ipc_buffer_mut(|ib| ib.inner_mut().seL4_Poll(ep.bits()));
//    match badge {
//        0 => Option::None,
//        _ => Option::Some((MessageInfo::from_inner(msg), badge)),
//    }
//}
/// poll 一个 [Notification]
///
/// 这是一个异步操作，当 poll 的时候会检查 [Notification] 是否有 signal，如果有则返回相关的 badge，
/// 如果没有，则返回 0 作为 badge
///
/// **TIPS: 如果创建一个默认的 [Notification] 之后没有设置的话，那么处于 unbadged 状体，即便有也是 None**
pub fn poll_notification(noti: Notification) -> Poll<u64> {
    let (_, badge) = with_ipc_buffer_mut(|ib| ib.inner_mut().seL4_Poll(noti.bits()));
    match badge {
        0 => Poll::Pending,
        _ => Poll::Ready(badge),
    }
}

/// poll 一个 [Endpoint]
///
/// 这是一个异步操作，当 poll 的时候会检查 [Endpoint] 是否有消息，如果有则返回相关的 消息和badge，
/// 如果没有，则返回 0 作为 badge
///
/// **TIPS: 如果创建一个默认的 [Endpoint] 之后没有设置的话，那么处于 unbadged 状体，即便有也是 None**
pub fn poll_endpoint(ep: Endpoint) -> Poll<(MessageInfo, u64)> {
    let (msg, badge) = with_ipc_buffer_mut(|ib| ib.inner_mut().seL4_Poll(ep.bits()));
    match badge {
        0 => Poll::Pending,
        _ => Poll::Ready((MessageInfo::from_inner(msg), badge)),
    }
}
