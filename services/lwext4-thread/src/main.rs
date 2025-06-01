#![no_std]
#![no_main]

extern crate alloc;
extern crate lwext4_thread;

use common::{
    config::{DEFAULT_SERVE_EP, IPC_DATA_LEN, REG_LEN},
    ipcrw::IpcBufferRW,
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
            let channel_id = with_ipc_buffer_mut(|ib| ib.msg_regs()[0] as _);
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
            // TODO: Open Directory
            let mut offset = 0;
            let flags = u32::read_buffer(ib, &mut offset);
            let path = <&str>::read_buffer(ib, &mut offset);
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
            let (inode, offset) = (ib.msg_regs()[0], ib.msg_regs()[1] as _);
            let buf_len = ib.msg_regs()[2] as usize;
            let addr = mapped.get(badge as usize).unwrap().0;

            let buffer = unsafe { core::slice::from_raw_parts_mut(addr as _, buf_len) };

            ib.msg_regs_mut()[0] = fs.read_at(inode, offset, buffer) as _;
            sel4::reply(ib, rev_msg.length(1).build());
        }
        FSIfaceEvent::write_at => {
            let (inode, offset) = (ib.msg_regs()[0], ib.msg_regs()[1] as _);
            let data_len = ib.msg_regs()[2] as usize;
            let data = ib.msg_bytes()[3 * REG_LEN..3 * REG_LEN + data_len].to_vec();

            ib.msg_regs_mut()[0] = fs.write_at(inode, offset, &data) as _;
            sel4::reply(ib, rev_msg.length(1).build());
        }
        FSIfaceEvent::mkdir => {
            let path = <&str>::read_buffer(ib, &mut 0);
            fs.mkdir(&path);
            sel4::reply(ib, rev_msg.build());
        }
        FSIfaceEvent::unlink => {
            let path = <&str>::read_buffer(ib, &mut 0);
            fs.unlink(&path);
            sel4::reply(ib, rev_msg.build());
        }
        FSIfaceEvent::close => {
            let index = ib.msg_regs()[0] as usize;
            fs.close(index);
            sel4::reply(ib, rev_msg.build());
        }
        FSIfaceEvent::stat => {
            let inode = ib.msg_regs()[0] as usize;
            let stat = fs.stat(inode);
            let len = size_of::<Stat>() / REG_LEN;
            unsafe {
                (ib.msg_bytes_mut().as_ptr() as *mut Stat).copy_from(&stat, 1);
            }
            sel4::reply(ib, rev_msg.length(len).build());
        }
        FSIfaceEvent::getdents64 => {
            let inode = ib.msg_regs()[0];
            let offset = ib.msg_regs()[1] as usize;
            let rlen = (ib.msg_regs()[2] as usize).min(IPC_DATA_LEN - 2 * REG_LEN);
            let buf = &mut ib.msg_bytes_mut()[2 * REG_LEN..][..rlen];

            let (real_rlen, offset) = fs.getdents64(inode, offset, buf);
            ib.msg_regs_mut()[0] = real_rlen as _;
            ib.msg_regs_mut()[1] = offset as _;
            sel4::reply(ib, rev_msg.length(2 + real_rlen.div_ceil(REG_LEN)).build());
        }
    }
}
