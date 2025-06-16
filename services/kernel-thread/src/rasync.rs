//! 异步相关的函数和工具
//!
//!

use core::{
    pin::Pin,
    task::{Context, Poll},
};

/// 一个简单的异步 Yield 实现
pub struct YieldNow {
    yielded: bool,
}

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            cx.waker().wake_by_ref(); // 重新调度自己
            Poll::Pending
        }
    }
}

/// 自定义 async yield_now
pub fn yield_now() -> YieldNow {
    YieldNow { yielded: false }
}
