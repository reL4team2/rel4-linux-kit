#![allow(missing_docs)]
use alloc::boxed::Box;
use common::root::create_channel;
use srv_gate::BLK_IMPLS;
use vfscore::BlockDevice;

const BLOCK_SIZE: usize = 0x200;

pub struct BlockDev;

impl BlockDevice for BlockDev {
    fn read_block(&self, block: usize, buffer: &mut [u8]) -> vfscore::VfsResult<usize> {
        assert_eq!(buffer.len() % BLOCK_SIZE, 0);
        assert!(buffer.len() <= 0x4000);
        let ptr = 0x3_0000_0000 as *const u8;
        BLK_IMPLS[0]
            .lock()
            .read_block(block, buffer.len() / BLOCK_SIZE);
        unsafe {
            ptr.copy_to_nonoverlapping(buffer.as_mut_ptr(), buffer.len());
        }
        Ok(buffer.len())
    }

    fn write_block(&self, block: usize, buf: &[u8]) -> vfscore::VfsResult<usize> {
        assert_eq!(buf.len() % BLOCK_SIZE, 0);
        assert!(buf.len() <= 0x4000);
        let ptr = 0x3_0000_0000 as *mut u8;
        unsafe {
            ptr.copy_from_nonoverlapping(buf.as_ptr(), buf.len());
        }
        BLK_IMPLS[0]
            .lock()
            .write_block(block, buf.len() / BLOCK_SIZE);
        Ok(buf.len())
    }

    fn capacity(&self) -> vfscore::VfsResult<u64> {
        Ok(BLK_IMPLS[0].lock().capacity())
    }
}

pub fn get_blk_dev() -> Box<dyn BlockDevice> {
    let channel_id = create_channel(0x3_0000_0000, 4);
    BLK_IMPLS[0].lock().init(channel_id);
    Box::new(BlockDev)
}
