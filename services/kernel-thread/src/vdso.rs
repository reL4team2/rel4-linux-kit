//! VDSO 资源管理和相关函数

use alloc::vec::Vec;
use common::config::PAGE_SIZE;
use sel4::cap::Granule;
use spin::Mutex;

use crate::{
    consts::task::{VDSO_AREA_SIZE, VDSO_REGION_KADDR},
    utils::{obj::alloc_vdso_page, page::map_page_self},
};

static VDSO_CAPS: Mutex<Vec<Granule>> = Mutex::new(Vec::new());

/// 初始化 VDSO 地址
pub fn init_vdso_addr() {
    for i in 0..VDSO_AREA_SIZE / PAGE_SIZE {
        let page = alloc_vdso_page();
        map_page_self(VDSO_REGION_KADDR + i * PAGE_SIZE, page);
        VDSO_CAPS.lock().push(page);
    }
}

/// 获取 VDSO 对应的 [Granule]
pub fn get_vdso_caps() -> Vec<Granule> {
    VDSO_CAPS.lock().clone()
}
