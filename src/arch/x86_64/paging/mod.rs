pub mod entry;
pub(crate) mod mmio;
mod mtrr;
mod page;
mod pat;
mod table;

use entry::EntryFlags;
use log::{info, trace};
pub use table::{ActiveP4Table, P4Table};

use crate::{
    arch::x86_64::{
        paging::pat::{read_pat_msr, write_pat_msr, MemoryType},
        rdmsr, wrmsr,
    },
    locks::SpinLock,
    mem::{align_down, align_up, frame::Frame, PhysicalAddress, VirtualAddress, PAGE_SIZE},
    multiboot::{MemMapEntryType, MultibootInfo},
};

const _1_GI_B: u64 = 1 * 1024u64.pow(3);
const _2_MI_B: u64 = 2 * 1024u64.pow(2);
const _4_KI_B: u64 = 4 * 1024u64.pow(1);

pub static ACTIVE_PAGETABLE: SpinLock<ActiveP4Table> = ActiveP4Table::locked();

pub(super) fn init(multiboot_info: &MultibootInfo) {
    if mtrr::supports_mtrr() {
        info!("Supports MTRR");
        let caps = unsafe { mtrr::read_mtrr_cap_msr() };
        info!("Capabilities: {:#x?}", caps);
        let def_type = unsafe { mtrr::read_mtrr_default_type_reg() };
        info!("Default Type: {:#x?}", def_type);
        if caps.supports_fixed_range_regs && def_type.fixed_range_mtrr_enabled {
            let vga_start = PhysicalAddress::new(0xb8000);
            let mt = unsafe { mtrr::read_fixed_range_mtrr(vga_start) };
            info!("MTRR Fixed reg for VGA: {:#x?}", mt);
            // unsafe { mtrr::write_fixed_range_mtrr(vga_start, mtrr::MemoryType::WriteCombining)};
            let mt = mtrr::MemoryTypes([mtrr::MemoryType::WriteCombining; 8]);
            unsafe { wrmsr(0x259, mt.into()) }
            info!("changed MTRR Fixed reg for VGA to: {:#x?}", unsafe {
                rdmsr(0x259)
            });
        }
    }

    if pat::supports_pat() {
        info!("supports PAT");
        let mut pas = unsafe { read_pat_msr() };
        info!("PAT {:#x?}", pas);
        pas.0[1] = MemoryType::WriteCombining;
        unsafe { write_pat_msr(pas) };
        info!("PAT {:#x?}", unsafe { read_pat_msr() });
    }

    // TODO: properly map the sections respecting their permissions

    // TODO: relying on firmware setting up the MTRRs correctly for MMIO. Map the MMIO pages to UC using PAT
    // Create a new `Mmio` struct and use RAII to remove paging entries on drop

    let mut new_page_table = P4Table::new();

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
                    _1_GI_B
                }
                MapSize::_2MiB => {
                    trace!("allocating 2MiB huge page");
                    let flags_to_set = flags_to_set | EntryFlags::HUGE_PAGE;
                    new_page_table.map_huge_2MiB(virt_addr, phys_addr, flags_to_set);
                    _2_MI_B
                }
                MapSize::_4KiB => {
                    trace!("allocating 4KiB page");
                    new_page_table.map_4KiB(virt_addr, phys_addr, flags_to_set);
                    _4_KI_B
                }
            };

            cur_start += cur_size;
            rem_size -= cur_size;
        }
    }

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
            EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::WRITE_THROUGH,
        );
    }

    // TODO: EBDA to MMIO????
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

    let mut guard = ACTIVE_PAGETABLE.lock();
    guard.switch(new_page_table);
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
    if start % _1_GI_B == 0 && size >= _1_GI_B {
        MapSize::_1GiB
    } else if start % _2_MI_B == 0 && size >= _2_MI_B {
        MapSize::_2MiB
    } else if start % _4_KI_B == 0 && size >= _4_KI_B {
        MapSize::_4KiB
    } else {
        unreachable!(
            "page is the smallest granularity of mapping, start: {:#x?}, size: {}",
            start, size
        );
    }
}
