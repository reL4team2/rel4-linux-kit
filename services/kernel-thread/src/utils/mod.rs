pub mod obj;
pub mod service;

use common::page::PhysPage;
use crate_consts::GRANULE_SIZE;
use memory_addr::{MemoryAddr, VirtAddr, PAGE_SIZE_4K};
use sel4::{init_thread, Cap, CapRights, VmAttributes};
use syscalls::Errno;

use crate::{page_seat_vaddr, syscall::SysResult, task::Sel4Task, FREE_PAGE_PLACEHOLDER};

#[macro_export]
macro_rules! test_func {
    ($title: literal, $test:block) => {{
        $crate::utils::print_test($title);
        $test;
        $crate::utils::print_test_end($title);
    }};
    ($title: literal, $test:expr) => {
        test_func!($title, { $test })
    };
}

/// Align a with b bits
pub fn align_bits<T: Into<usize> + From<usize>>(a: T, b: usize) -> T {
    (a.into() & !((1 << b) - 1)).into()
}

#[repr(C, align(4096))]
pub struct FreePagePlaceHolder(#[allow(dead_code)] pub [u8; GRANULE_SIZE]);

pub unsafe fn init_free_page_addr() -> usize {
    core::ptr::addr_of!(FREE_PAGE_PLACEHOLDER) as _
}

fn process_item_list<T: Sized, F>(
    task: &Sel4Task,
    addr: VirtAddr,
    number: Option<usize>,
    mut f: F,
) -> SysResult
where
    F: FnMut(VirtAddr, usize, usize),
{
    let mut buf_addr = addr;
    if number.is_none() && core::mem::size_of::<T>() + addr.align_offset_4k() > PAGE_SIZE_4K {
        return Err(Errno::EINVAL);
    }
    let number = number.unwrap_or(1);
    let mut len = core::mem::size_of::<T>() * number;
    while len > 0 {
        if let Some(cap) = task.mapped_page.get(&align_bits(buf_addr.as_usize(), 12)) {
            let new_cap = PhysPage::new(Cap::<sel4::cap_type::SmallPage>::from_bits(0));
            init_thread::slot::CNODE
                .cap()
                .absolute_cptr(new_cap.cap())
                .copy(
                    &init_thread::slot::CNODE.cap().absolute_cptr(cap.cap()),
                    CapRights::all(),
                )
                .unwrap();

            new_cap
                .cap()
                .frame_map(
                    init_thread::slot::VSPACE.cap(),
                    page_seat_vaddr(),
                    CapRights::all(),
                    VmAttributes::DEFAULT,
                )
                .unwrap();
            let copy_len = (PAGE_SIZE_4K - buf_addr.align_offset_4k()).min(len);
            f(
                VirtAddr::from_usize(page_seat_vaddr() + buf_addr.align_offset_4k()),
                buf_addr - addr,
                copy_len,
            );
            len -= copy_len;
            buf_addr += copy_len;

            new_cap.cap().frame_unmap().unwrap();
            init_thread::slot::CNODE
                .cap()
                .absolute_cptr(new_cap.cap())
                .delete()
                .unwrap();
        } else {
            return Err(Errno::EFAULT);
        }
    }
    Ok(buf_addr - addr)
}

// TODO: pass the capabilites to the kernel thread
pub(crate) fn read_item<T: Sized + Copy>(task: &Sel4Task, addr: *const T) -> Result<T, Errno> {
    let mut item: T = unsafe { core::mem::MaybeUninit::zeroed().assume_init() };
    process_item_list::<T, _>(task, VirtAddr::from_ptr_of(addr), None, |src, _, _| {
        item = unsafe { core::ptr::read_volatile(src.as_ptr() as *const T) }
    })?;
    Ok(item)
}

pub(crate) fn write_item<T: Sized + Copy>(task: &Sel4Task, addr: *const T, item: &T) -> SysResult {
    process_item_list::<T, _>(
        task,
        VirtAddr::from_ptr_of(addr),
        None,
        |dst, _, copy_len| unsafe {
            core::ptr::copy_nonoverlapping(item as *const T, dst.as_mut_ptr() as *mut T, copy_len);
        },
    )
}

pub(crate) fn read_item_list<T: Sized + Copy>(
    task: &Sel4Task,
    addr: *const T,
    num: Option<usize>,
    buf: &mut [T],
) -> SysResult {
    process_item_list::<T, _>(
        task,
        VirtAddr::from_ptr_of(addr),
        num,
        |src, offset, copy_len| {
            // let bytes =
            //     unsafe { core::slice::from_raw_parts(buf_addr.as_ptr(), offset + copy_len) };
            unsafe {
                core::ptr::copy_nonoverlapping(
                    src.as_ptr() as *const T,
                    buf.as_mut_ptr().add(offset),
                    copy_len,
                );
            }
        },
    )
}

/// Write items to the given address.
///
/// # Arguments
///
/// - buf: The buffer to write.
/// - addr: The address to write to.
pub(crate) fn write_item_list<T: Sized + Copy>(
    task: &Sel4Task,
    addr: *mut T,
    num: Option<usize>,
    buf: &[T],
) -> SysResult {
    process_item_list::<u8, _>(
        task,
        VirtAddr::from_ptr_of(addr),
        num,
        |dst, offset, copy_len| unsafe {
            core::ptr::copy_nonoverlapping(
                buf.as_ptr().add(offset),
                dst.as_mut_ptr() as *mut T,
                copy_len,
            );
        },
    )
}
