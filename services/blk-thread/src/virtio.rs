use alloc::collections::BTreeMap;
use core::{
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};
use crate_consts::DMA_ADDR_START;
use sel4::{self, debug_println};
use spin::Mutex;
use virtio_drivers::{BufferDirection, Hal, PhysAddr, PAGE_SIZE};

use crate::ROOT_SERVICE;

static DMA_ADDR: AtomicUsize = AtomicUsize::new(DMA_ADDR_START);
static ADDR_MAP: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new());

pub fn translate_address(vaddr: usize) -> usize {
    let vp_index = vaddr / PAGE_SIZE;
    let offset = vaddr % PAGE_SIZE;

    let mut map = ADDR_MAP.lock();
    let paddr = match map.get(&vp_index) {
        Some(v) => v * PAGE_SIZE + offset,
        None => {
            let paddr = ROOT_SERVICE
                .translate_addr(vaddr as *const u8 as _)
                .expect("can't translate address");

            map.insert(vp_index, paddr / PAGE_SIZE);
            paddr
        }
    };
    log::debug!("Translate: {:#x} -> {:#x}", vaddr, paddr);
    paddr
}

pub struct HalImpl;

unsafe impl Hal for HalImpl {
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (PhysAddr, NonNull<u8>) {
        debug_println!("[BlockThread] DMA Alloc Page: {}", pages);
        let vaddr = DMA_ADDR.load(Ordering::Acquire);
        DMA_ADDR.store(vaddr + pages * PAGE_SIZE, Ordering::Release);

        log::debug!("allocated ptr: {:#x?}", vaddr);
        // let paddr = ROOT_SERVICE
        //     .translate_addr(vaddr)
        //     .expect("can't translate address");
        // (paddr, NonNull::new(vaddr as *mut u8).unwrap())
        (
            translate_address(vaddr),
            NonNull::new(vaddr as *mut u8).unwrap(),
        )
    }

    unsafe fn dma_dealloc(_paddr: PhysAddr, _vaddr: NonNull<u8>, _pages: usize) -> i32 {
        0
    }

    unsafe fn mmio_phys_to_virt(_paddr: PhysAddr, _size: usize) -> NonNull<u8> {
        todo!()
    }

    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> PhysAddr {
        // ROOT_SERVICE
        //     .translate_addr(buffer.as_ptr() as *const u8 as _)
        //     .expect("can't translate address")
        translate_address(buffer.as_ptr() as *const u8 as _)
    }

    unsafe fn unshare(_paddr: PhysAddr, _buffer: NonNull<[u8]>, _direction: BufferDirection) {
        // Nothing to do, as the host already has access to all memory and we didn't copy the buffer
        // anywhere else.
    }
}
