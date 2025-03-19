//! 文件系统管道模块
//!
//!

use core::{
    cmp,
    sync::atomic::{
        AtomicU64,
        Ordering::{self, SeqCst},
    },
};

use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use spin::Mutex;
use syscalls::Errno;

use super::vfs::FileInterface;

/// 管道
pub struct Pipe {
    /// 如果 tx 为 true，那么就是只写端口，否则只读
    tx: bool,
    /// 发送管道的数量
    tx_num: Arc<AtomicU64>,
    /// 管道数据
    queue: Arc<Mutex<VecDeque<u8>>>,
}

impl Drop for Pipe {
    fn drop(&mut self) {
        if self.tx {
            self.tx_num.fetch_sub(1, SeqCst);
        }
    }
}

impl FileInterface for Pipe {
    fn readat(&mut self, _off: usize, buf: &mut [u8]) -> super::vfs::FileResult<usize> {
        if self.tx {
            return Err(Errno::EACCES);
        }
        let mut queue = self.queue.lock();
        if queue.len() == 0 && self.tx_num.load(Ordering::SeqCst) > 0 {
            return Err(Errno::EAGAIN);
        }
        let rlen = cmp::min(buf.len(), queue.len());
        (0..rlen).for_each(|x| buf[x] = queue.pop_front().unwrap());
        Ok(rlen)
    }

    fn writeat(&mut self, _off: usize, data: &[u8]) -> super::vfs::FileResult<usize> {
        if !self.tx {
            return Err(Errno::EACCES);
        }
        let mut queue = self.queue.lock();
        data.iter().for_each(|x| queue.push_back(*x));
        Ok(data.len())
    }

    fn metadata(&self) -> super::vfs::FileResult<super::vfs::FileMetaData> {
        todo!()
    }

    fn stat(&self) -> super::vfs::FileResult<common::services::fs::Stat> {
        todo!()
    }
}

/// 创建一对管道
///
/// 返回的管道顺序位 (读， 写)
pub fn create_pipe2() -> (Pipe, Pipe) {
    let queue = Arc::new(Mutex::new(VecDeque::new()));
    let tx_num = Arc::new(AtomicU64::new(1));
    let rx_queue = Pipe {
        tx: false,
        tx_num: tx_num.clone(),
        queue: queue.clone(),
    };
    let tx_queue = Pipe {
        tx: true,
        tx_num,
        queue,
    };
    (rx_queue, tx_queue)
}
