//! 内存相关系统调用
//!
//!

use super::SysResult;
use crate::{
    consts::task::DEF_HEAP_ADDR,
    task::{
        Sel4Task,
        shm::{MapedSharedMemory, SHARED_MEMORY, SharedMemory},
    },
    utils::obj::alloc_untyped_unit,
};
use alloc::{sync::Arc, vec::Vec};
use common::{
    config::PAGE_SIZE,
    mem::CapMemSet,
    page::PhysPage,
    slot::{alloc_slot, recycle_slot},
};
use libc_core::mman::MapFlags;
use sel4::{Cap, CapRights, cap_type};
use sel4_kit::slot_manager::LeafSlot;
use spin::Mutex;
use syscalls::Errno;

#[inline]
pub(super) fn sys_brk(task: &Sel4Task, heap: usize) -> SysResult {
    debug!("BRK @ heap: {heap:#x}");
    Ok(task.brk(heap))
}

#[inline]
pub(super) fn sys_mmap(
    task: &Sel4Task,
    start: usize,
    size: usize,
    prot: usize,
    flags: usize,
    fd: isize,
    off: usize,
) -> SysResult {
    let flags = MapFlags::from_bits_truncate(flags as _);
    if flags.contains(MapFlags::SHARED) {
        log::warn!("mmap share is not supported now!");
    }
    assert_eq!(start % PAGE_SIZE, 0);
    debug!("MMAP @ {start:#x} {size:#x} {prot:#x} {flags:#x} {fd:#x} {off:#x}");
    warn!("mmap is just map a regular page RWX");
    if task.mem.lock().heap >= start + size && start >= DEF_HEAP_ADDR {
        warn!("Only supported the case that calling brk before");
        return Ok(start);
    }
    let start = task.find_free_area(start, size);
    if fd > 0 {
        let file = task
            .file
            .file_ds
            .lock()
            .get(fd as _)
            .ok_or(Errno::EINVAL)?
            .clone();
        let file_len = file.file_size()?;
        let mut data = vec![0u8; file_len];
        file.readat(0, &mut data)?;
        for addr in (start..start + size).step_by(PAGE_SIZE) {
            task.map_blank_page(addr);
        }
        task.write_bytes(start, &data);
    } else {
        for addr in (start..start + size).step_by(PAGE_SIZE) {
            task.map_blank_page(addr);
        }
    }
    Ok(start)
}

pub(super) fn sys_munmap(task: &Sel4Task, start: usize, len: usize) -> SysResult {
    debug!("sys_munmap @ start: {:#x}, len: {:#x}", start, len);
    task.mem.lock().mapped_page.retain(|vaddr, x| {
        if (start..start + len).contains(vaddr) {
            x.cap().frame_unmap().unwrap();
            let slot = LeafSlot::from_cap(x.cap());
            slot.revoke().unwrap();
            slot.delete().unwrap();
            recycle_slot(slot);
            false
        } else {
            true
        }
    });
    Ok(0)
}

pub(super) fn sys_shmget(
    _task: &Sel4Task,
    mut key: usize,
    size: usize,
    shmflg: usize,
) -> SysResult {
    debug!(
        "sys_shmget @ key: {}, size: {}, shmflg: {:#o}",
        key, size, shmflg
    );
    if key == 0 {
        key = SHARED_MEMORY.lock().keys().cloned().max().unwrap_or(0) + 1;
    }
    let mem = SHARED_MEMORY.lock().get(&key).cloned();
    if mem.is_some() {
        return Ok(key);
    }
    if shmflg & 0o1000 > 0 {
        let capset = Mutex::new(CapMemSet::new(Some(alloc_untyped_unit)));
        let vector: Vec<Cap<cap_type::Granule>> = (0..size.div_ceil(PAGE_SIZE))
            .map(|_| capset.lock().alloc_page())
            .collect();
        SHARED_MEMORY
            .lock()
            .insert(key, Arc::new(SharedMemory::new(capset, vector)));
        return Ok(key);
    }
    Err(Errno::ENOENT)
}

pub(super) fn sys_shmat(task: &Sel4Task, shmid: usize, shmaddr: usize, shmflg: usize) -> SysResult {
    debug!(
        "sys_shmat @ shmid: {}, shmaddr: {}, shmflg: {:#o}",
        shmid, shmaddr, shmflg
    );

    let trackers = SHARED_MEMORY.lock().get(&shmid).cloned();
    if trackers.is_none() {
        return Err(Errno::ENOENT);
    }
    let trackers = trackers.unwrap();

    let vaddr = task.find_free_area(shmaddr, trackers.trackers.len() * PAGE_SIZE);
    let vaddr = if shmaddr == 0 { vaddr } else { shmaddr };

    for (i, page) in trackers.trackers.iter().enumerate() {
        let new_slot = alloc_slot();
        new_slot
            .copy_from(&LeafSlot::from_cap(*page), CapRights::all())
            .unwrap();
        task.map_page(vaddr + i * PAGE_SIZE, PhysPage::new(new_slot.cap()));
    }

    task.shm.lock().push(Arc::new(MapedSharedMemory {
        key: shmid,
        mem: SHARED_MEMORY.lock().get(&shmid).unwrap().clone(),
        start: vaddr,
        size: trackers.trackers.len() * PAGE_SIZE,
    }));

    Ok(vaddr)
}

pub(super) fn sys_shmctl(_task: &Sel4Task, shmid: usize, cmd: usize, arg: usize) -> SysResult {
    debug!("sys_shmctl @ shmid: {}, cmd: {}, arg: {}", shmid, cmd, arg);

    if cmd == 0 {
        if let Some(map) = SHARED_MEMORY.lock().get_mut(&shmid) {
            *map.deleted.lock() = true;
        }
        return Ok(0);
    }
    Err(Errno::EPERM)
}
