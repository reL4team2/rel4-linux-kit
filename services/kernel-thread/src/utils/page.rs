//! 页表处理工具模块
//!

use sel4::{CapRights, Error, VmAttributes, cap::SmallPage, init_thread::slot};

use super::obj::alloc_pt;

/// 映射一个页到指定的地址
///
/// - `vaddr` 需要映射的地址
/// - `page`  需要映射的页能力
pub fn map_page_self(vaddr: usize, page: SmallPage) {
    for _ in 0..sel4::vspace_levels::NUM_LEVELS {
        let res: core::result::Result<(), sel4::Error> = page.frame_map(
            slot::VSPACE.cap(),
            vaddr as _,
            CapRights::all(),
            VmAttributes::DEFAULT,
        );
        match res {
            Ok(_) => return,
            Err(Error::FailedLookup) => {
                let pt_cap = alloc_pt();
                pt_cap
                    .pt_map(slot::VSPACE.cap(), vaddr, VmAttributes::DEFAULT)
                    .unwrap();
            }
            _ => res.unwrap(),
        }
    }
}
