#![no_std]
#![no_main]

extern crate alloc;
extern crate lwext4_thread;

use common::{
    config::{DEFAULT_SERVE_EP, REG_LEN},
    read_types, reply_with,
    root::join_channel,
};
use flatten_objects::FlattenObjects;
use sel4::{IpcBuffer, MessageInfoBuilder, with_ipc_buffer_mut};
use sel4_runtime::utils::alloc_free_addr;
use srv_gate::fs::{FSIface, FSIfaceEvent, Stat};

#[sel4_runtime::main]
fn main() {
    log::info!("Booting...");

    // let channel_id = create_channel(0x3_0000_0000, 4);
    // BLK_IMPLS[0].lock().init(channel_id);

    let mut mapped = FlattenObjects::<(usize, usize), 32>::new();
    let mut fs = lwext4_thread::EXT4FS.lock();

    loop {
        with_ipc_buffer_mut(|ib| handle_events(ib, &mut *fs, &mut mapped));
    }
}

fn handle_events(
    ib: &mut IpcBuffer,
    fs: &mut (dyn FSIface + 'static),
    mapped: &mut FlattenObjects<(usize, usize), 32>,
) {
    let (message, badge) = DEFAULT_SERVE_EP.recv(());
    let rev_msg = MessageInfoBuilder::default();

    let msg_label = FSIfaceEvent::try_from(message.label()).unwrap();
    log::debug!("Recv <{:?}> len: {}", msg_label, message.length());
    match msg_label {
        FSIfaceEvent::init => {
            let channel_id = read_types!(usize);

            let ptr = alloc_free_addr(0) as *mut u8;
            let size = join_channel(channel_id, ptr as usize);
            alloc_free_addr(size);
            mapped
                .add_at(badge as _, (ptr as usize, channel_id))
                .map_err(|_| ())
                .unwrap();
            fs.init(channel_id, 0, 0);
            sel4::reply(ib, rev_msg.build());
        }
        FSIfaceEvent::open => {
            let (flags, path) = read_types!(ib, u32, &str);
            match fs.open(&path, flags) {
                Ok((index, size)) => {
                    ib.msg_regs_mut()[0] = index as _;
                    ib.msg_regs_mut()[1] = size as _;
                    sel4::reply(ib, rev_msg.length(2).build());
                }
                Err(errno) => {
                    ib.msg_regs_mut()[0] = errno.into_raw() as _;
                    sel4::reply(ib, rev_msg.length(1).build());
                }
            }
        }
        FSIfaceEvent::read_at => {
            let (inode, offset, buf_len) = read_types!(ib, u64, usize, usize);

            let addr = mapped.get(badge as usize).unwrap().0;

            let buffer = unsafe { core::slice::from_raw_parts_mut(addr as _, buf_len) };

            reply_with!(ib, fs.read_at(inode, offset, buffer));
        }
        FSIfaceEvent::write_at => {
            let (inode, offset, data) = read_types!(ib, u64, usize, &[u8]);

            reply_with!(ib, fs.write_at(inode, offset, &data))
        }
        FSIfaceEvent::mkdir => {
            let path = read_types!(ib, &str);
            fs.mkdir(&path);
            sel4::reply(ib, rev_msg.build());
        }
        FSIfaceEvent::unlink => {
            let path = read_types!(ib, &str);
            fs.unlink(&path);
            sel4::reply(ib, rev_msg.build());
        }
        FSIfaceEvent::close => {
            let index = read_types!(ib, usize);
            fs.close(index);
            sel4::reply(ib, rev_msg.build());
        }
        FSIfaceEvent::stat => {
            let inode = read_types!(ib, usize);

            let stat = fs.stat(inode);
            let len = size_of::<Stat>() / REG_LEN;
            unsafe {
                (ib.msg_bytes_mut().as_ptr() as *mut Stat).copy_from(&stat, 1);
            }
            sel4::reply(ib, rev_msg.length(len).build());
        }
        FSIfaceEvent::getdents64 => {
            let (inode, offset, mut buf) = read_types!(ib, u64, usize, &[u8]);

            let (real_rlen, offset) = fs.getdents64(inode, offset, buf.as_mut_slice());
            ib.msg_regs_mut()[0] = real_rlen as _;
            ib.msg_regs_mut()[1] = offset as _;
            sel4::reply(ib, rev_msg.length(2 + real_rlen.div_ceil(REG_LEN)).build());
        }
    }
}
