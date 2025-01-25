use common::{STDERR_FD, STDOUT_FD};
use memory_addr::{MemoryAddr, VirtAddr, PAGE_SIZE_4K};
use sel4::{init_thread, Cap, CapRights, VmAttributes};
use syscalls::Errno;

use crate::{child_test::TASK_MAP, page_seat_vaddr, syscall::SysResult, utils::align_bits};

pub(crate) fn sys_write(badge: u64, fd: i32, buf: *const u8, mut count: usize) -> SysResult {
    let task_map = TASK_MAP.lock();
    let task = task_map.get(&badge).unwrap();
    if fd != STDOUT_FD && fd != STDERR_FD {
        return Err(Errno::ENOSYS);
    }
    let mut buf_addr = VirtAddr::from_ptr_of(buf);
    let mut payload_length = 0;
    while count > 0 {
        if let Some(cap) = task.mapped_page.get(&align_bits(buf_addr.as_usize(), 12)) {
            let new_cap = Cap::<sel4::cap_type::SmallPage>::from_bits(0);
            init_thread::slot::CNODE
                .cap()
                .absolute_cptr(new_cap)
                .copy(
                    &init_thread::slot::CNODE.cap().absolute_cptr(cap.cap()),
                    CapRights::all(),
                )
                .unwrap();

            new_cap
                .frame_map(
                    init_thread::slot::VSPACE.cap(),
                    page_seat_vaddr(),
                    CapRights::all(),
                    VmAttributes::DEFAULT,
                )
                .unwrap();
            let copy_len = (PAGE_SIZE_4K - buf_addr.align_offset_4k()).min(count);
            let bytes = unsafe {
                core::slice::from_raw_parts(
                    page_seat_vaddr() as *const u8,
                    copy_len + buf_addr.align_offset_4k(),
                )
            };

            // FIXME: ensure that data in the page.z
            bytes[buf_addr.align_offset_4k()..(buf_addr.align_offset_4k() + copy_len)]
                .iter()
                .map(u8::clone)
                .for_each(sel4::sys::seL4_DebugPutChar);

            count -= copy_len;
            buf_addr += copy_len;
            payload_length += copy_len;
            new_cap.frame_unmap().unwrap();
            init_thread::slot::CNODE
                .cap()
                .absolute_cptr(new_cap)
                .delete()
                .unwrap();
        }
    }
    Ok(payload_length)
}
