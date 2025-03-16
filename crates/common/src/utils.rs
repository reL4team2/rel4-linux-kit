extern crate alloc;
use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use config::PAGE_SIZE;
use core::{cmp, ops::Range};
use object::{
    Object, ObjectSegment, SegmentFlags,
    elf::{PF_R, PF_W, PF_X},
};
use sel4::{
    MessageInfoBuilder, cap::Endpoint, init_thread::slot, with_ipc_buffer, with_ipc_buffer_mut,
};

use crate::{
    ObjectAllocator,
    consts::{IPC_DATA_LEN, REG_LEN},
    page::PhysPage,
};
// 计算 elf image 的虚地址空间范围
pub fn footprint<'a>(image: &'a impl Object<'a>) -> Range<usize> {
    let min: usize = image
        .segments()
        .map(|seg| seg.address())
        .min()
        .unwrap()
        .try_into()
        .unwrap();
    let max: usize = image
        .segments()
        .map(|seg| seg.address() + seg.size())
        .max()
        .unwrap()
        .try_into()
        .unwrap();
    coarsen_footprint(&(min..max), PAGE_SIZE)
}

// 将ELF的虚地址空间 map 到页表中，但不分配物理页
pub fn map_intermediate_translation_tables(
    allocator: &mut ObjectAllocator,
    vspace: sel4::cap::VSpace,
    footprint: Range<usize>,
) {
    for level in 1..sel4::vspace_levels::NUM_LEVELS {
        let span_bytes = 1 << sel4::vspace_levels::span_bits(level);
        let footprint_at_level = coarsen_footprint(&footprint, span_bytes);
        for i in 0..(footprint_at_level.len() / span_bytes) {
            let ty = sel4::TranslationTableObjectType::from_level(level).unwrap();
            let addr = footprint_at_level.start + i * span_bytes;
            allocator
                .allocate_and_retype(ty.blueprint())
                .cast::<sel4::cap_type::UnspecifiedIntermediateTranslationTable>()
                .generic_intermediate_translation_table_map(
                    ty,
                    vspace,
                    addr,
                    sel4::VmAttributes::default(),
                )
                .unwrap()
        }
    }
}

/// 将 ELF image 映射到物理页
pub fn map_image<'a>(
    allocator: &mut ObjectAllocator,
    mapped_page: &mut BTreeMap<usize, PhysPage>,
    vspace: sel4::cap::VSpace,
    footprint: Range<usize>,
    image: &'a impl Object<'a>,
) {
    // 计算需要的物理页数
    let num_pages = footprint.len() / PAGE_SIZE;
    let mut pages = (0..num_pages)
        .map(|_| (allocator.alloc_page(), sel4::CapRightsBuilder::all()))
        .collect::<Vec<(sel4::cap::Granule, sel4::CapRightsBuilder)>>();

    for seg in image.segments() {
        let segment_addr = usize::try_from(seg.address()).unwrap();
        let segment_size = usize::try_from(seg.size()).unwrap();
        let segment_footprint =
            coarsen_footprint(&(segment_addr..(segment_addr + segment_size)), PAGE_SIZE);
        let num_pages_spanned_by_segment = segment_footprint.len() / PAGE_SIZE;
        let segment_data_size = seg.data().unwrap().len();
        let segment_data_footprint = coarsen_footprint(
            &(segment_addr..(segment_addr + segment_data_size)),
            PAGE_SIZE,
        );
        let num_pages_spanned_by_segment_data = segment_data_footprint.len() / PAGE_SIZE;
        let segment_page_index_offset = (segment_footprint.start - footprint.start) / PAGE_SIZE;

        for (_, rights) in &mut pages[segment_page_index_offset..][..num_pages_spanned_by_segment] {
            add_rights(rights, seg.flags());
        }

        let mut data = seg.data().unwrap();
        let mut offset_into_page = segment_addr % PAGE_SIZE;
        for (page_cap, _) in
            &pages[segment_page_index_offset..][..num_pages_spanned_by_segment_data]
        {
            let data_len = (PAGE_SIZE - offset_into_page).min(data.len());

            // 映射物理页到 root-task 的虚拟地址空间，并且将数据拷贝到物理页中
            let phys_page = PhysPage::new(*page_cap);

            phys_page.lock()[offset_into_page..offset_into_page + data_len]
                .copy_from_slice(&data[..data_len]);

            data = &data[data_len..];
            offset_into_page = 0;
        }
    }

    // 将物理页映射到 child 的虚拟地址空间
    for (i, (page_cap, rights)) in pages.into_iter().enumerate() {
        let addr = footprint.start + i * PAGE_SIZE;
        page_cap
            .frame_map(vspace, addr, rights.build(), sel4::VmAttributes::DEFAULT)
            .unwrap();
        mapped_page.insert(addr, PhysPage::new(page_cap));
    }
}

fn add_rights(rights: &mut sel4::CapRightsBuilder, flags: SegmentFlags) {
    match flags {
        SegmentFlags::Elf { p_flags } => {
            if p_flags & PF_R != 0 {
                *rights = rights.read(true);
            }
            if p_flags & PF_W != 0 {
                *rights = rights.write(true);
            }
            if p_flags & PF_X != 0 {
                *rights = rights.grant(true);
            }
        }
        _ => unimplemented!(),
    }
}

fn coarsen_footprint(footprint: &Range<usize>, granularity: usize) -> Range<usize> {
    round_down(footprint.start, granularity)..footprint.end.next_multiple_of(granularity)
}

const fn round_down(n: usize, b: usize) -> usize {
    n - n % b
}

/// 初始化接收 Slot
pub fn init_recv_slot() {
    with_ipc_buffer_mut(|ipc_buffer| {
        ipc_buffer.set_recv_slot(&slot::CNODE.cap().absolute_cptr_from_bits_with_depth(0, 64));
    })
}

/// 发送大量数据
///
/// - `ep`   发送数据使用的 [Endpoint]
/// - `msg`  发送数据使用的消息，使用 [MessageInfoBuilder], 发送长度由此函数填充
/// - `data` 需要发送的数据
pub fn send_bulk_data(ep: Endpoint, msg: MessageInfoBuilder, data: &[u8]) {
    let mut start = 0;
    while start < data.len() {
        let send_size = cmp::min(IPC_DATA_LEN - 2 * REG_LEN, data.len() - start);
        with_ipc_buffer_mut(|ib| {
            // 剩下的数据量
            ib.msg_regs_mut()[0] = (data.len() - start - send_size) as _;
            // 本次发送的数据量
            ib.msg_regs_mut()[1] = send_size as _;
            // 发送的数据
            ib.msg_bytes_mut()[2 * REG_LEN..2 * REG_LEN + send_size]
                .copy_from_slice(&data[start..start + send_size]);
        });
        start += send_size;
        let ret = ep.call(msg.length(send_size.div_ceil(REG_LEN) + 2).build());
        assert!(ret.label() == 0);
    }
}

/// 接收大量数据
///
/// - `ep`         接收数据使用的 [Endpoint]
/// - `msg_label`  接收数据时的标签，用来判断是否是异常数据
pub fn recv_bulk_data(ep: Endpoint, msg_label: usize) -> Vec<u8> {
    let mut recv_data = Vec::new();
    loop {
        let (msg, _) = ep.recv(());
        assert!(msg.label() as usize == msg_label);
        let last = with_ipc_buffer(|ib| {
            let len = ib.msg_regs()[1] as usize;
            recv_data.extend_from_slice(&ib.msg_bytes()[2 * REG_LEN..2 * REG_LEN + len]);
            ib.msg_regs()[0]
        });
        if last == 0 {
            break;
        }
    }
    recv_data
}
