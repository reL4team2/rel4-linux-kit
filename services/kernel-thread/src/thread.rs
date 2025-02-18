use alloc::boxed::Box;
use sel4::{
    debug_println, init_thread, set_ipc_buffer, CNodeCapData, CapTypeForFrameObjectOfFixedSize,
    Error, VmAttributes,
};
use sel4_elf_header::{ElfHeader, PT_TLS};
use sel4_initialize_tls::{TlsImage, TlsReservationLayout, UncheckedTlsImage};
use sel4_root_task::{abort, panicking::catch_unwind};
use sel4_stack::Stack;

use crate::utils::obj::{alloc_notification, alloc_page, alloc_pt, alloc_tcb};

static SECONDARY_THREAD_STACK: Stack<4096> = Stack::new();

const SECONDARY_THREAD_IPC_BUFFER_ADDR: usize = 0x10_0000_0000;

#[allow(unused)]
pub fn test_threads() {
    let ntfn = alloc_notification();
    let thread_tcb = alloc_tcb();

    let secondary_thread_ipc_buffer_cap = alloc_page();

    loop {
        match secondary_thread_ipc_buffer_cap.frame_map(
            init_thread::slot::VSPACE.cap(),
            SECONDARY_THREAD_IPC_BUFFER_ADDR,
            sel4::CapRights::all(),
            sel4::VmAttributes::default(),
        ) {
            Ok(()) => {
                debug_println!("[RootTask] map device memory success");
                break;
            }
            Err(Error::FailedLookup) => {
                debug_println!("[RootTask] map device memory failed, try to allocate page table");
                let pt = alloc_pt();
                pt.pt_map(
                    init_thread::slot::VSPACE.cap(),
                    SECONDARY_THREAD_IPC_BUFFER_ADDR,
                    VmAttributes::default(),
                )
                .unwrap();
            }
            Err(e) => {
                panic!("[RootTask] map device memory failed: {:?}", e);
            }
        }
    }

    thread_tcb
        .tcb_configure(
            init_thread::slot::NULL.cptr(),
            init_thread::slot::CNODE.cap(),
            CNodeCapData::new(0, 0),
            init_thread::slot::VSPACE.cap(),
            SECONDARY_THREAD_IPC_BUFFER_ADDR as u64,
            secondary_thread_ipc_buffer_cap,
        )
        .unwrap();

    let thread_fn: ThreadFn = ThreadFn::new(move || {
        let ipc_buffer = {
            SECONDARY_THREAD_IPC_BUFFER_ADDR
                .next_multiple_of(sel4::cap_type::Granule::FRAME_OBJECT_TYPE.bytes())
                as *mut sel4::IpcBuffer
        };
        unsafe { set_ipc_buffer(ipc_buffer.as_mut().unwrap()) };
        debug_println!("Secondary thread started");
        ntfn.signal();
        debug_println!("Secondary thread say Goodbye");
        thread_tcb.tcb_suspend().unwrap();
        unreachable!();
    });

    thread_tcb
        .tcb_write_all_registers(true, &mut make_user_context(thread_fn))
        .unwrap();

    ntfn.wait();
    debug_println!("Primary thread received notification from secondary thread");
    debug_println!("Secondary thread TEST PASSED");
}

struct ThreadFn(Box<dyn FnOnce() -> ! + core::panic::UnwindSafe + Send + 'static>);

impl ThreadFn {
    fn new(f: impl FnOnce() -> ! + core::panic::UnwindSafe + Send + 'static) -> Self {
        Self(Box::new(f))
    }

    fn run(self) -> ! {
        (self.0)()
    }

    fn into_arg(self) -> sel4::Word {
        Box::into_raw(Box::new(self)) as sel4::Word
    }

    unsafe fn from_arg(arg: sel4::Word) -> Self {
        *Box::from_raw(arg as *mut Self)
    }
}

fn make_user_context(f: ThreadFn) -> sel4::UserContext {
    let mut ctx = sel4::UserContext::default();

    *ctx.sp_mut() = (SECONDARY_THREAD_STACK.bottom().ptr() as usize)
        .try_into()
        .unwrap();
    *ctx.pc_mut() = (thread_entrypoint as usize).try_into().unwrap();
    *ctx.c_param_mut(0) = f.into_arg();

    let tls_reservation = TlsReservation::new(&get_tls_image());
    *(&mut ctx.inner_mut().tpidr_el0) = tls_reservation.thread_pointer() as sel4::Word;
    core::mem::forget(tls_reservation);

    ctx
}

unsafe extern "C" fn thread_entrypoint(arg: sel4::Word) -> ! {
    let f = ThreadFn::from_arg(arg);
    let _ = catch_unwind(|| f.run());
    abort!("Secondary thread panic!")
}

struct TlsReservation {
    start: *mut u8,
    layout: TlsReservationLayout,
}

impl TlsReservation {
    fn new(tls_image: &TlsImage) -> Self {
        let layout = tls_image.reservation_layout();
        let start = unsafe { ::alloc::alloc::alloc(layout.footprint()) };
        unsafe {
            tls_image.initialize_reservation(start);
        };
        Self { start, layout }
    }

    fn thread_pointer(&self) -> usize {
        (self.start as usize) + self.layout.thread_pointer_offset()
    }
}

impl Drop for TlsReservation {
    fn drop(&mut self) {
        unsafe {
            ::alloc::alloc::dealloc(self.start, self.layout.footprint());
        }
    }
}

fn get_tls_image() -> TlsImage {
    extern "C" {
        static __ehdr_start: ElfHeader;
    }
    let phdrs = unsafe {
        assert!(__ehdr_start.check_magic());
        __ehdr_start.locate_phdrs()
    };
    let phdr = phdrs.iter().find(|phdr| phdr.p_type == PT_TLS).unwrap();
    let unchecked = UncheckedTlsImage {
        vaddr: phdr.p_vaddr,
        filesz: phdr.p_filesz,
        memsz: phdr.p_memsz,
        align: phdr.p_align,
    };
    unchecked.check().unwrap()
}
