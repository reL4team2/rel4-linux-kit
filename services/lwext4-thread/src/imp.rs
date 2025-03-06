use lwext4_rust::KernelDevOp;

use crate::BLK_SERVICE;

const BLOCK_SIZE: usize = 0x200;

pub struct Ext4Disk {
    block_id: usize,
    offset: usize,
}

impl KernelDevOp for Ext4Disk {
    type DevType = Self;

    fn write(dev: &mut Self::DevType, mut buf: &[u8]) -> Result<usize, i32> {
        let mut write_len = 0;
        while !buf.is_empty() {
            match dev.write_one(buf) {
                Ok(0) => break,
                Ok(n) => {
                    buf = &buf[n..];
                    write_len += n;
                }
                Err(_e) => return Err(-1),
            }
        }
        Ok(write_len)
    }

    fn read(dev: &mut Self::DevType, mut buf: &mut [u8]) -> Result<usize, i32> {
        let mut read_len = 0;
        while !buf.is_empty() {
            match dev.read_one(buf) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                    read_len += n;
                }
                Err(_e) => return Err(-1),
            }
        }
        Ok(read_len)
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

    fn read_one(&mut self, buf: &mut [u8]) -> Result<usize, i32> {
        let read_size = if self.offset == 0 && buf.len() >= BLOCK_SIZE {
            // whole block
            BLK_SERVICE
                .read_block(self.block_id, &mut buf[..BLOCK_SIZE])
                .unwrap();
            self.block_id += 1;
            BLOCK_SIZE
        } else {
            // partial block
            let mut data = [0u8; BLOCK_SIZE];
            let start = self.offset;
            let count = buf.len().min(BLOCK_SIZE - self.offset);
            if start > BLOCK_SIZE {
                log::debug!("block size: {} start {}", BLOCK_SIZE, start);
            }

            BLK_SERVICE.read_block(self.block_id, &mut data).unwrap();
            buf[..count].copy_from_slice(&data[start..start + count]);

            self.offset += count;
            if self.offset >= BLOCK_SIZE {
                self.block_id += 1;
                self.offset -= BLOCK_SIZE;
            }
            count
        };
        Ok(read_size)
    }

    fn write_one(&mut self, buf: &[u8]) -> Result<usize, i32> {
        let write_size = if self.offset == 0 && buf.len() >= BLOCK_SIZE {
            // whole block
            BLK_SERVICE
                .write_block(self.block_id, &buf[..BLOCK_SIZE])
                .unwrap();
            self.block_id += 1;
            BLOCK_SIZE
        } else {
            // partial block
            let mut data = [0u8; BLOCK_SIZE];
            let start = self.offset;
            let count = buf.len().min(BLOCK_SIZE - self.offset);

            BLK_SERVICE.read_block(self.block_id, &mut data).unwrap();
            data[start..start + count].copy_from_slice(&buf[..count]);
            BLK_SERVICE.write_block(self.block_id, &data).unwrap();

            self.offset += count;
            if self.offset >= BLOCK_SIZE {
                self.block_id += 1;
                self.offset -= BLOCK_SIZE;
            }
            count
        };
        Ok(write_size)
    }
}
