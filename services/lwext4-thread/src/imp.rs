use lwext4_rust::KernelDevOp;

use crate::BLK_SERVICE;

const BLOCK_SIZE: usize = 0x200;

pub struct Ext4Disk {
    block_id: usize,
    offset: usize,
}

impl KernelDevOp for Ext4Disk {
    type DevType = Self;

    fn write(dev: &mut Self::DevType, buf: &[u8]) -> Result<usize, i32> {
        dev.write_one(buf)
    }

    fn read(dev: &mut Self::DevType, buf: &mut [u8]) -> Result<usize, i32> {
        dev.read_one(buf)
    }

    fn seek(dev: &mut Self::DevType, off: i64, whence: i32) -> Result<i64, i32> {
        let size = BLK_SERVICE.capacity().unwrap() as i64;
        let new_pos = match whence as u32 {
            lwext4_rust::bindings::SEEK_SET => Some(off),
            lwext4_rust::bindings::SEEK_CUR => {
                dev.position().checked_add_signed(off).map(|v| v as i64)
            }
            lwext4_rust::bindings::SEEK_END => size.checked_add(off),
            _ => {
                log::error!("invalid seek() whence: {}", whence);
                Some(off)
            }
        }
        .ok_or(-1)?;

        if new_pos as u64 > (size as _) {
            panic!("exceed position");
        }

        dev.set_position(new_pos as u64);
        Ok(new_pos)
    }

    fn flush(_dev: &mut Self::DevType) -> Result<usize, i32>
    where
        Self: Sized,
    {
        todo!()
    }
}

impl Ext4Disk {
    /// Create a new disk.
    pub fn new() -> Self {
        Self {
            block_id: 0,
            offset: 0,
        }
    }

    /// Get the position of the cursor.
    pub fn position(&self) -> u64 {
        (self.block_id * BLOCK_SIZE + self.offset) as u64
    }

    /// Set the position of the cursor.
    pub fn set_position(&mut self, pos: u64) {
        self.block_id = pos as usize / BLOCK_SIZE;
        self.offset = pos as usize % BLOCK_SIZE;
    }

    #[inline]
    fn read_one(&mut self, buf: &mut [u8]) -> Result<usize, i32> {
        assert_eq!(buf.len() % BLOCK_SIZE, 0);
        assert_eq!(self.offset, 0);
        assert!(buf.len() <= 0x4000);
        let ptr = 0x3_0000_0000 as *const u8;
        BLK_SERVICE
            .read_block(self.block_id, buf.len() / BLOCK_SIZE)
            .unwrap();
        unsafe {
            ptr.copy_to_nonoverlapping(buf.as_mut_ptr(), buf.len());
        }
        self.set_position(self.position() + buf.len() as u64);
        Ok(buf.len())
    }

    #[inline]
    fn write_one(&mut self, buf: &[u8]) -> Result<usize, i32> {
        assert_eq!(buf.len() % BLOCK_SIZE, 0);
        assert_eq!(self.offset, 0);
        assert!(buf.len() <= 0x4000);
        let ptr = 0x3_0000_0000 as *mut u8;
        unsafe {
            ptr.copy_from_nonoverlapping(buf.as_ptr(), buf.len());
        }
        BLK_SERVICE
            .write_block(self.block_id, buf.len() / BLOCK_SIZE)
            .unwrap();
        self.set_position(self.position() + buf.len() as u64);
        Ok(buf.len())
    }
}
