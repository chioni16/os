pub mod entry;
mod mmio;
mod page;
mod table;

pub(super) use mmio::Mmio;

use entry::EntryFlags;
use log::trace;
pub use table::{ActiveP4Table, P4Table};

use crate::{
    locks::SpinLock,
    mem::{align_down, align_up, frame::Frame, PhysicalAddress, VirtualAddress, PAGE_SIZE},
    multiboot::{MemMapEntryType, MultibootInfo},
};

const _1GiB: u64 = 1 * 1024u64.pow(3);
const _2MiB: u64 = 2 * 1024u64.pow(2);
const _4KiB: u64 = 4 * 1024u64.pow(1);

pub static ACTIVE_PAGETABLE: SpinLock<ActiveP4Table> = ActiveP4Table::locked();

pub(super) fn init(multiboot_info: &MultibootInfo) {
    // TODO: properly map the sections respecting their permissions

    // TODO: relying on firmware setting up the MTRRs correctly for MMIO. Map the MMIO pages to UC using PAT
    // Create a new `MMIO` struct and use RAII to remove paging entries on drop

    trace!("before new_table init");
    let mut new_page_table = P4Table::new();
    trace!("after new_table init");

    for region in multiboot_info.multiboot_mem_tags().unwrap() {
        trace!("region: {:#x?}", region);
    }

    // let ram_regions = multiboot_info.multiboot_mem_tags().unwrap()
    //     .clone()
    //     .filter(|region| region.entry_type() == MemMapEntryType::Ram);
    for region in multiboot_info.multiboot_mem_tags().unwrap() {
        trace!(
            "region: start: {:#x?}, end: {:#x?}",
            region.start(),
            region.end()
        );
        let mut cur_start = align_down(region.start().to_inner(), PAGE_SIZE);
        let mut rem_size = region.end().to_inner() - cur_start;
        rem_size = align_up(rem_size, PAGE_SIZE);
        trace!("start: {:#x?}, size: {:#x?}", cur_start, rem_size);

        let mut flags_to_set = EntryFlags::PRESENT | EntryFlags::WRITABLE;
        if region.entry_type() != MemMapEntryType::Ram {
            flags_to_set |= EntryFlags::WRITE_THROUGH | EntryFlags::NO_CACHE;
        }

        while rem_size > 0 {
            let phys_addr = PhysicalAddress::new(cur_start);
            let virt_addr = unsafe { phys_addr.to_virt().unwrap() };
            let cur_size = match find_best_fit(cur_start, rem_size) {
                MapSize::_1GiB => {
                    trace!("allocating 1GiB huge page");
                    let flags_to_set = flags_to_set | EntryFlags::HUGE_PAGE;
                    new_page_table.map_huge_1GiB(virt_addr, phys_addr, flags_to_set);
                    _1GiB
                }
                MapSize::_2MiB => {
                    trace!("allocating 2MiB huge page");
                    let flags_to_set = flags_to_set | EntryFlags::HUGE_PAGE;
                    new_page_table.map_huge_2MiB(virt_addr, phys_addr, flags_to_set);
                    _2MiB
                }
                MapSize::_4KiB => {
                    trace!("allocating 4KiB page");
                    new_page_table.map_4KiB(virt_addr, phys_addr, flags_to_set);
                    _4KiB
                }
            };

            cur_start += cur_size;
            rem_size -= cur_size;
        }
        // let flags_to_set = EntryFlags::PRESENT | EntryFlags::WRITABLE;
        // for frame in Frame::range_inclusive(
        //     Frame::containing_address(region.start()),
        //     Frame::containing_address(region.end()),
        // ) {
        //     let phys_addr = frame.start_address();
        //     let virt_addr = unsafe { phys_addr.to_virt().unwrap() };
        //     new_page_table.map(virt_addr, phys_addr, flags_to_set);
        // }
    }
    // let elf_sections = multiboot_info
    //     .multiboot_elf_tags()
    //     .unwrap()
    //     .filter(|s| s.flags().contains(Elf64SectionFlags::SHF_ALLOC));
    // for section in elf_sections {
    //     trace!("section: start: {:#x?}, end: {:#x?}", section.start(), section.end());
    //     // let flags_to_set = EntryFlags::from_elf_section_flags(&section);
    //     let flags_to_set = EntryFlags::PRESENT | EntryFlags::WRITABLE;
    //     let start_virtual = section.start().to_inner();
    //     let end_virtual = section.end().to_inner();
    //     let start_physical =
    //         PhysicalAddress::new(start_virtual - unsafe { addr_of!(HIGHER_HALF) } as u64);
    //     let end_physical =
    //         PhysicalAddress::new(end_virtual - unsafe { addr_of!(HIGHER_HALF) } as u64);
    //     for frame in Frame::range_inclusive(
    //         Frame::containing_address(start_physical),
    //         Frame::containing_address(end_physical),
    //     ) {
    //         let phys_addr = frame.start_address();
    //         let virt_addr = unsafe { phys_addr.to_virt().unwrap() };
    //         new_page_table.map(virt_addr, phys_addr, flags_to_set);
    //     }
    // }

    // // map frames used by allocator

    // TODO: VGA mem move this to MMIO
    for frame in Frame::range_inclusive(
        &Frame::containing_address(PhysicalAddress::new(0xA0000)),
        &Frame::containing_address(PhysicalAddress::new(0xBFFFF)),
    ) {
        let phys_addr = frame.start_address();
        let virt_addr = unsafe { phys_addr.to_virt().unwrap() };
        new_page_table.map_4KiB(
            virt_addr,
            phys_addr,
            EntryFlags::PRESENT
                | EntryFlags::WRITABLE
                | EntryFlags::WRITE_THROUGH
                | EntryFlags::NO_CACHE,
        );
    }

    // // TODO: EBDA to MMIO????
    for frame in Frame::range_inclusive(
        &Frame::containing_address(PhysicalAddress::new(0xE0000)),
        &Frame::containing_address(PhysicalAddress::new(0xFFFFF)),
    ) {
        let phys_addr = frame.start_address();
        let virt_addr = unsafe { phys_addr.to_virt().unwrap() };
        new_page_table.map_4KiB(
            virt_addr,
            phys_addr,
            EntryFlags::PRESENT
                | EntryFlags::WRITABLE
                | EntryFlags::WRITE_THROUGH
                | EntryFlags::NO_CACHE,
        );
    }

    trace!("after new_table map");
    let mut guard = ACTIVE_PAGETABLE.lock();
    let a = guard.switch(new_page_table);
    drop(guard);
    trace!("switched");
    trace!("lock: {:#x?}", ACTIVE_PAGETABLE);
    trace!("old_p4: {:#x?}", a);
}

pub fn translate_using_current_page_table(virt_addr: VirtualAddress) -> Option<PhysicalAddress> {
    let mut guard = ACTIVE_PAGETABLE.lock();
    guard.translate(virt_addr)
}

pub fn map_rw_using_current_page_table(virt_addr: VirtualAddress, phys_addr: PhysicalAddress) {
    let mut guard = ACTIVE_PAGETABLE.lock();
    guard.map_4KiB(
        virt_addr,
        phys_addr,
        EntryFlags::WRITABLE | EntryFlags::PRESENT,
    );
}
pub fn unmap_rw_using_current_page_table(virt_addr: VirtualAddress) {
    let mut guard = ACTIVE_PAGETABLE.lock();
    guard.unmap(virt_addr);
}

enum MapSize {
    _1GiB,
    _2MiB,
    _4KiB,
}

fn find_best_fit(start: u64, size: u64) -> MapSize {
    // first check alignment and then if the region is of sufficient size
    if start % _1GiB == 0 && size >= _1GiB {
        MapSize::_1GiB
    } else if start % _2MiB == 0 && size >= _2MiB {
        MapSize::_2MiB
    } else if start % _4KiB == 0 && size >= _4KiB {
        MapSize::_4KiB
    } else {
        unreachable!(
            "page is the smallest granularity of mapping, start: {:#x?}, size: {}",
            start, size
        );
    }
}
