//! 内存系统调用相关的定义
//!
//!

bitflags::bitflags! {
    #[derive(Debug)]
    pub(crate) struct MapFlags: u32 {
        const MAP_SHARED          =    0x01;
        const MAP_PRIVATE         =    0x02;
        const MAP_SHARED_VALIDATE =    0x03;
        const MAP_TYPE            =    0x0f;
        const MAP_FIXED           =    0x10;
        const MAP_ANONYMOUS       =    0x20;
        const MAP_NORESERVE       =    0x4000;
        const MAP_GROWSDOWN       =    0x0100;
        const MAP_DENYWRITE       =    0x0800;
        const MAP_EXECUTABLE      =    0x1000;
        const MAP_LOCKED          =    0x2000;
        const MAP_POPULATE        =    0x8000;
        const MAP_NONBLOCK        =    0x10000;
        const MAP_STACK           =    0x20000;
        const MAP_HUGETLB         =    0x40000;
        const MAP_SYNC            =    0x80000;
        const MAP_FIXED_NOREPLACE =    0x100000;
        const MAP_FILE            =    0;
    }
}
