#![no_std]
#![no_main]

#[macro_use]
extern crate alloc;

use alloc::{string::String, sync::Arc, vec::Vec};

use common::services::{block::BlockService, fs::FileEvent, root::find_service};
use crate_consts::DEFAULT_SERVE_EP;
use ext4_rs::{BlockDevice, Ext4, Ext4DirEntry, Ext4File};
use sel4::{debug_print, debug_println, with_ipc_buffer_mut, MessageInfoBuilder};
use slot_manager::LeafSlot;

sel4_runtime::entry_point!(main);

const BLK_SERVICE: BlockService = BlockService::from_bits(0x21);

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

    fn write_offset(&self, _offset: usize, _data: &[u8]) {
        todo!()
    }
}

fn main() -> ! {
    common::init_log!(log::LevelFilter::Debug);
    common::init_recv_slot();

    log::info!("Booting...");

    find_service("block-thread", LeafSlot::new(0x21)).unwrap();

    BLK_SERVICE.ping().unwrap();
    log::info!("Found Block Thread, It reply ping message");

    // 创建 Ext4 文件系统
    let disk = Arc::new(Ext4Disk);
    let ext4 = Ext4::open(disk);

    // log::debug!("step 1");

    // let mut file = Ext4File::new();
    // let ret = ext4.ext4_open_new(&mut file, "123.txt", "r+", true);

    // let mut rlen = file.fsize as usize;
    // let mut file_content = vec![0u8; file.fsize as _];
    // ext4.ext4_file_read(&mut file, &mut file_content, rlen as _, &mut rlen);
    // log::debug!("content: {:?}", String::from_utf8(file_content));

    // log::debug!("ret: {:?}", ret);
    // log::error!("hello world!");

    // loop {}

    let rev_msg = MessageInfoBuilder::default();
    loop {
        let (message, _) = DEFAULT_SERVE_EP.recv(());
        let msg_label = FileEvent::from(message.label());
        log::debug!("Recv <{:?}> len: {}", msg_label, message.length());
        match msg_label {
            FileEvent::Ping => {
                with_ipc_buffer_mut(|ib| {
                    sel4::reply(ib, rev_msg.build());
                });
            }
            // FIXME: 应该返回一个结构，或者数组表示所有文件
            // 类似于 getdents
            FileEvent::ReadDir => {
                log::debug!("Read Dir Message");
                let dir_entry: Vec<Ext4DirEntry> = ext4.read_dir_entry(2);
                for entry in dir_entry {
                    debug_print!("{}\t", entry.get_name());
                }
                debug_println!();
                with_ipc_buffer_mut(|ipc_buffer| sel4::reply(ipc_buffer, rev_msg.build()));
            }
            FileEvent::Unknown(label) => {
                log::warn!("Unknown label: {}", label);
            }
        }
    }
}
