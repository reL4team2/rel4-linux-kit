use common::services::root::RootService;
use crate_consts::DEFAULT_PARENT_EP;
use slot_manager::LeafSlot;

use super::obj::alloc_slot;

/// root-task 的服务接口 [LeafSlot]
const ROOT_SERVICE: RootService = RootService::from_bits(DEFAULT_PARENT_EP);

#[inline]
pub fn find_service(name: &str) -> Result<LeafSlot, ()> {
    let slot = alloc_slot();
    ROOT_SERVICE.find_service(name, slot)?;
    Ok(slot)
}
