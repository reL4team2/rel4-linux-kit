use core::cmp;

use alloc::vec::Vec;
use ext4_rs::BlockDevice;

use crate::BLK_SERVICE;

const BLOCK_SIZE: usize = 4096;
const TRANS_LIMIT: usize = 0x200;

pub struct Ext4Disk;

impl BlockDevice for Ext4Disk {
    fn read_offset(&self, offset: usize) -> Vec<u8> {
        let mut buf = vec![0; BLOCK_SIZE];

        let start_block_id = offset / TRANS_LIMIT;
        let mut offset_in_block = offset % TRANS_LIMIT;

        let mut total_bytes_read = 0;
        let mut data = vec![0u8; TRANS_LIMIT];

        for i in 0..(BLOCK_SIZE / TRANS_LIMIT) {
            let current_block_id = start_block_id + i;

            let bytes_to_copy = match total_bytes_read {
                0 => TRANS_LIMIT - offset_in_block,
                _ => TRANS_LIMIT,
            };

            let buf_start = total_bytes_read;
            let buf_end = buf_start + bytes_to_copy;

            match offset_in_block {
                0 => BLK_SERVICE
                    .read_block(current_block_id, &mut buf[buf_start..buf_end])
                    .unwrap(),
                _ => {
                    BLK_SERVICE.read_block(current_block_id, &mut data).unwrap();
                    buf[buf_start..buf_end]
                        .copy_from_slice(&data[offset_in_block..(offset_in_block + bytes_to_copy)]);
                    offset_in_block = 0; // only the first block has an offset within the block
                }
            }

            total_bytes_read += bytes_to_copy;
        }

        buf
    }

    fn write_offset(&self, offset: usize, data: &[u8]) {
        let mut buf = [0; TRANS_LIMIT];

        let mut wlen = 0;

        loop {
            let block_id = offset / TRANS_LIMIT;
            let offset_in_block = offset % TRANS_LIMIT;
            log::warn!("wlen: {}  all data: {}", wlen, data.len());

            if offset_in_block == 0 && data.len() - wlen > TRANS_LIMIT {
                BLK_SERVICE
                    .write_block(block_id, &data[wlen..wlen + TRANS_LIMIT])
                    .unwrap();
                wlen += TRANS_LIMIT;
            } else {
                BLK_SERVICE.read_block(block_id, &mut buf).unwrap();
                let this_wlen = cmp::min(data.len() - wlen, TRANS_LIMIT - offset_in_block);
                buf[offset_in_block..offset_in_block + this_wlen]
                    .copy_from_slice(&data[wlen..wlen + this_wlen]);
                BLK_SERVICE.write_block(block_id, &buf).unwrap();
                wlen += this_wlen;
            }
            if wlen >= data.len() {
                break;
            }
        }
    }
}
