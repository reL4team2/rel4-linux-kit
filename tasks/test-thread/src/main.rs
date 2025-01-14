#![no_std]
#![no_main]
#![feature(naked_functions)]
#[macro_use]
extern crate alloc;
extern crate sel4_panicking;
sel4_panicking_env::register_debug_put_char!(sel4::sys::seL4_DebugPutChar);

use core::{
    arch::naked_asm,
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
};

use common::{CloneArgs, CustomMessageLabel};
use crate_consts::DEFAULT_PARENT_EP;
use sel4::{
    cap::Endpoint, debug_println, set_ipc_buffer, with_ipc_buffer_mut, Cap,
    CapTypeForFrameObjectOfFixedSize, MessageInfo,
};
use sel4_dlmalloc::{StaticDlmallocGlobalAlloc, StaticHeap};
use sel4_sync::PanickingRawMutex;
use syscalls::Sysno;

/// Load current tls register.
pub(crate) fn load_tp_reg() -> usize {
    let mut tp: usize;
    unsafe {
        core::arch::asm!("mrs {0}, tpidr_el0", out(reg) tp);
    }
    tp
}

/// Save the tls register
pub(crate) fn set_tp_reg(tp: usize) {
    unsafe {
        core::arch::asm!("msr tpidr_el0, {0}", in(reg) tp);
    }
}
/// The entry of the test thread component.
#[no_mangle]
#[naked]
unsafe extern "C" fn _start() -> () {
    naked_asm!(
        "
            bl     {main}
            mov    x1, 0
            msr    tpidr_el0, x1
            blr    x0
            b      .
        ",
        main = sym main
    )
}

const WORD_SIZE: usize = core::mem::size_of::<usize>();

/// vsyscall handler.
pub fn vsyscall_handler(
    id: usize,
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    e: usize,
    f: usize,
) -> usize {
    #[inline(always)]
    fn get_tid() -> u64 {
        // Write syscall registers to ipc buffer.
        with_ipc_buffer_mut(|buffer| {
            let msgs: &mut [u64] = buffer.msg_regs_mut();
            msgs[0] = Sysno::gettid.id() as _;
        });
        // Load endpoint and send SysCall message.
        let ep = Cap::from_bits(EP_CPTR.load(Ordering::SeqCst));
        let _ = ep.call(MessageInfo::new(
            CustomMessageLabel::SysCall.to_label(),
            0,
            0,
            7 * WORD_SIZE,
        ));
        with_ipc_buffer_mut(|buffer| buffer.msg_regs()[0])
    }

    debug_println!("syscall id: {}", id);
    let prev_id = if id == Sysno::clone.id() as _ {
        get_tid()
    } else {
        0
    };

    let tp = load_tp_reg();
    // Restore the TLS register used by Shim components.
    set_tp_reg(TP_REG.load(Ordering::SeqCst));

    // Write syscall registers to ipc buffer.
    with_ipc_buffer_mut(|buffer| {
        let msgs: &mut [u64] = buffer.msg_regs_mut();
        msgs[0] = id as _;
        msgs[1] = a as _;
        msgs[2] = b as _;
        msgs[3] = c as _;
        msgs[4] = d as _;
        msgs[5] = e as _;
        msgs[6] = f as _;
    });
    // Load endpoint and send SysCall message.
    let ep = Cap::from_bits(EP_CPTR.load(Ordering::SeqCst));
    let message = ep.call(MessageInfo::new(
        CustomMessageLabel::SysCall.to_label(),
        0,
        0,
        7 * WORD_SIZE,
    ));

    if prev_id != 0 {
        set_tp_reg(tp);
        if get_tid() != prev_id {
            return 0;
        }
    }

    // Ensure that has one WORD_SIZE contains result.
    assert_eq!(message.length(), WORD_SIZE);

    // Get the result of the fake syscall
    let ret = with_ipc_buffer_mut(|buffer| buffer.msg_regs()[0]);

    // Restore The TLS Register used by linux App
    set_tp_reg(tp);
    debug_println!("syscall id: {} ret: {}", id, ret);
    ret as usize
}

/// TLS register of shim component, use it to restore in [vsyscall_handler]
pub(crate) static TP_REG: AtomicUsize = AtomicUsize::new(0);
/// Endpoint cptr
pub(crate) static EP_CPTR: AtomicU64 = AtomicU64::new(0);

const STACK_SIZE: usize = 0x18000;
sel4_runtime_common::declare_stack!(STACK_SIZE);

const HEAP_SIZE: usize = 0x18000;
static STATIC_HEAP: StaticHeap<HEAP_SIZE> = StaticHeap::new();

#[global_allocator]
static GLOBAL_ALLOCATOR: StaticDlmallocGlobalAlloc<
    PanickingRawMutex,
    &'static StaticHeap<HEAP_SIZE>,
> = StaticDlmallocGlobalAlloc::new(PanickingRawMutex::new(), &STATIC_HEAP);

/// The main entry of the shim component
fn main(_ep: Endpoint, busybox_entry: usize, vsyscall_section: usize) -> usize {
    // Display Debug information
    debug_println!("[User] busybox entry: {:#x}", busybox_entry);
    debug_println!(
        "[User] vyscall section: {:#x} -> {:#x}",
        vsyscall_section,
        vsyscall_handler as usize
    );

    set_ipc_buffer_with_symbol();
    let ep = Endpoint::from_bits(DEFAULT_PARENT_EP);
    // Store Tls reg and endpoint cptr
    TP_REG.store(load_tp_reg(), Ordering::SeqCst);
    EP_CPTR.store(ep.bits(), Ordering::SeqCst);

    // Test Send Custom Message
    ep.call(MessageInfo::new(
        CustomMessageLabel::TestCustomMessage.to_label(),
        0,
        0,
        0,
    ));

    debug_println!("[User] send ipc buffer done");

    let mmap_ptr = vsyscall_handler(
        Sysno::mmap.id() as usize,
        0x1000,
        0x1000,
        0b11,
        0b10000,
        0,
        0,
    );

    let content = "Hello, World!";

    unsafe {
        core::ptr::copy_nonoverlapping(content.as_ptr(), mmap_ptr as *mut u8, content.len());
    }
    let _ = vsyscall_handler(
        Sysno::write.id() as usize,
        1,
        mmap_ptr as usize,
        content.len(),
        0,
        0,
        0,
    );

    let clone_args = CloneArgs::default();

    let ret = vsyscall_handler(
        Sysno::clone.id() as usize,
        (&clone_args) as *const _ as usize,
        0,
        0,
        0,
        0,
        0,
    );
    if ret != 0 {
        debug_println!("Child task: {} created", ret);
        vsyscall_handler(Sysno::exit.id() as usize, 0, 0, 0, 0, 0, 0);
    } else {
        debug_println!("Hello, I am the child task");
        vsyscall_handler(Sysno::execve.id() as usize, 0, 0, 0, 0, 0, 0);
    }

    unreachable!()
    // let socket_id = vsyscall_handler(Sysno::socket.id() as usize, 0, 0, 0, 0, 0, 0);

    // let mut socket_addr = LibcSocketAddr {
    //     sa_family: 2,
    //     sa_data: [0; 14],
    // };
    // // Address is 0.0.0.0:6379;
    // socket_addr.sa_data[0] = (6379 >> 8) as u8;
    // socket_addr.sa_data[1] = (6379 & 0xff) as u8;

    // let _ = vsyscall_handler(
    //     Sysno::bind.id() as usize,
    //     socket_id,
    //     (&socket_addr as *const LibcSocketAddr) as usize,
    //     0,
    //     0,
    //     0,
    //     0,
    // );

    // debug_println!("bind done");

    // let _ = vsyscall_handler(Sysno::listen.id() as usize, socket_id, 0, 0, 0, 0, 0);

    // debug_println!("listen done");
    // loop {
    //     fn http_server(socket_id: usize) {
    //         debug_println!("Run a new connection");
    //         const CONTENT: &str = r#"<html>
    // <head>
    //   <title>Hello, ArceOS</title>
    // </head>
    // <body>
    //   <center>
    //     <h1>Hello, <a href="https://github.com/rcore-os/arceos">ArceOS</a></h1>
    //   </center>
    //   <hr>
    //   <center>
    //     <i>Powered by <a href="https://github.com/rcore-os/arceos/tree/main/apps/net/httpserver">ArceOS example HTTP server</a> v0.1.0</i>
    //   </center>
    // </body>
    // </html>
    // "#;

    //         macro_rules! header {
    //             () => {
    //                 "\
    // HTTP/1.1 200 OK\r\n\
    // Content-Type: text/html\r\n\
    // Content-Length: {}\r\n\
    // Connection: close\r\n\
    // \r\n\
    // {}"
    //             };
    //         }

    //         let mut requeset = [0u8; 256];
    //         let cnt = vsyscall_handler(
    //             Sysno::recvfrom.id() as usize,
    //             socket_id,
    //             (&mut requeset as *mut [u8; 256]) as usize,
    //             256,
    //             0,
    //             0,
    //             0,
    //         );
    //         // let cnt = stream.recv(&mut requeset).unwrap();
    //         debug_println!("[Net thread] Request size: {} buf: {:?}", cnt, requeset);
    //         let response_buf = format!(header!(), CONTENT.len(), CONTENT);
    //         // stream.send(response_buf.as_bytes()).unwrap();
    //         vsyscall_handler(
    //             Sysno::sendto.id() as usize,
    //             socket_id,
    //             response_buf.as_ptr() as usize,
    //             response_buf.len(),
    //             0,
    //             0,
    //             0,
    //         );
    //         debug_println!(
    //             "[Net thread] Send size: {} buf: {:?}",
    //             response_buf.len(),
    //             response_buf
    //         );
    //     }
    //     let mut new_socket_address = LibcSocketAddr {
    //         sa_family: 2,
    //         sa_data: [0; 14],
    //     };
    //     let new_socket_id = vsyscall_handler(
    //         Sysno::accept.id() as usize,
    //         socket_id,
    //         (&mut new_socket_address as *mut LibcSocketAddr) as usize,
    //         0,
    //         0,
    //         0,
    //         0,
    //     );

    //     debug_println!("accept done: {}", new_socket_id);
    //     if new_socket_id as isize > 0 {
    //         http_server(new_socket_id);
    //     }
    // }

    // // Return the true entry point
    // return busybox_entry;
}

/// Send a syscall to sel4 with none arguments
pub fn sys_null(sys: isize) {
    unsafe {
        core::arch::asm!(
            "svc 0",
            in("x7") sys,
        );
    }
}

fn set_ipc_buffer_with_symbol() {
    extern "C" {
        static _end: usize;
    }
    let ipc_buffer = unsafe {
        ((core::ptr::addr_of!(_end) as usize)
            .next_multiple_of(sel4::cap_type::Granule::FRAME_OBJECT_TYPE.bytes())
            as *mut sel4::IpcBuffer)
            .as_mut()
            .unwrap()
    };

    set_ipc_buffer(ipc_buffer);
}
