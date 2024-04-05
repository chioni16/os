extern crate alloc;
use alloc::boxed::Box;

use super::entry::{Entry, EntryFlags, PAGE_ENTRY_SIZE};
use super::{get_current_page_table, translate_using_current_page_table};
use crate::mem::frame::Frame;
use crate::mem::{PhysicalAddress, VirtualAddress, PAGE_SIZE};
use core::ops::{Index, IndexMut};

const PAGE_ENTRY_COUNT: u64 = PAGE_SIZE / PAGE_ENTRY_SIZE;

#[derive(Debug)]
#[repr(align(4096))]
pub struct Table {
    entries: [Entry; PAGE_ENTRY_COUNT as usize],
}

impl Table {
    pub unsafe fn new() -> Box<Self> {
        let mut p4 = Box::new(Self {
            entries: [Entry::zero(); 512],
        });
        p4[0].set(
            Self::alloc_page(),
            EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::USER_ACCESSIBLE,
        );

        let cur_p3 = get_current_page_table()[0].next_page_table();
        let p3 = p4[0].next_page_table_mut();
        // p3.zero();
        p3.entries[3] = cur_p3.entries[3].clone(); // copy over the entries 3, 4, 5, 6 from the equivalent
        p3.entries[4] = cur_p3.entries[3].clone(); // child PT that is currently in use
        p3.entries[5] = cur_p3.entries[3].clone(); // these correspond to the addresses our kernel uses
        p3.entries[6] = cur_p3.entries[3].clone(); // plus some more, so that the entire physical memory is mapped
        p4
    }

    unsafe fn alloc_page() -> Frame {
        let frame: Box<[Entry; 512]> = Box::new([Entry::zero(); 512]);
        let virt_addr = VirtualAddress::new(Box::into_raw(frame) as u64);
        let phys_addr = translate_using_current_page_table(virt_addr).unwrap();
        Frame::containing_address(phys_addr)
    }

    pub fn zero(&mut self) {
        for entry in &mut self.entries {
            entry.set_zero();
        }
    }

    // SAFETY: only call on p4 page tables
    // doesn't support huge pages as of now
    pub unsafe fn map(
        &mut self,
        virt_addr: VirtualAddress,
        phys_addr: PhysicalAddress,
        flags_to_set: EntryFlags,
        // allocator: &mut dyn FrameAllocator,
    ) {
        assert!(!flags_to_set.contains(EntryFlags::HUGE_PAGE));

        let p3 = &mut self[virt_addr.p4_index()];
        if !p3.flags().contains(EntryFlags::PRESENT) {
            // let new_frame = allocator.allocate_frame().unwrap();
            let new_frame = Self::alloc_page();
            p3.set(new_frame, EntryFlags::PRESENT);
        }
        if flags_to_set.contains(EntryFlags::WRITABLE) {
            p3.set_flags(EntryFlags::WRITABLE);
        }
        if flags_to_set.contains(EntryFlags::USER_ACCESSIBLE) {
            p3.set_flags(EntryFlags::USER_ACCESSIBLE);
        }

        let p2 = &mut p3.next_page_table_mut()[virt_addr.p3_index()];
        if !p2.flags().contains(EntryFlags::PRESENT) {
            // let new_frame = allocator.allocate_frame().unwrap();
            let new_frame = Self::alloc_page();
            p2.set(new_frame, EntryFlags::PRESENT);
        }
        if flags_to_set.contains(EntryFlags::WRITABLE) {
            p2.set_flags(EntryFlags::WRITABLE);
        }
        if flags_to_set.contains(EntryFlags::USER_ACCESSIBLE) {
            p2.set_flags(EntryFlags::USER_ACCESSIBLE);
        }

        let p1 = &mut p2.next_page_table_mut()[virt_addr.p2_index()];
        if !p1.flags().contains(EntryFlags::PRESENT) {
            // let new_frame = allocator.allocate_frame().unwrap();
            let new_frame = Self::alloc_page();
            p1.set(new_frame, EntryFlags::PRESENT);
        }
        if flags_to_set.contains(EntryFlags::WRITABLE) {
            p1.set_flags(EntryFlags::WRITABLE);
        }
        if flags_to_set.contains(EntryFlags::USER_ACCESSIBLE) {
            p1.set_flags(EntryFlags::USER_ACCESSIBLE);
        }

        let p0 = &mut p1.next_page_table_mut()[virt_addr.p1_index()];
        p0.set(Frame::containing_address(phys_addr), flags_to_set);
    }

    // SAFETY: only call on p4 page tables
    // &mut self as traverse requires it - find a workaround
    pub unsafe fn translate(&mut self, virt_addr: VirtualAddress) -> Option<PhysicalAddress> {
        self.traverse(virt_addr).map(|(phys_addr, _)| phys_addr)
    }

    // SAFETY: only call on p4 page tables
    pub unsafe fn unmap(&mut self, virt_addr: VirtualAddress) -> bool {
        if let Some((_, entry)) = self.traverse(virt_addr) {
            entry.unset_flags(EntryFlags::PRESENT);
            true
        } else {
            false
        }
    }

    unsafe fn traverse(
        &mut self,
        virt_addr: VirtualAddress,
    ) -> Option<(PhysicalAddress, &mut Entry)> {
        let p3 = &mut self[virt_addr.p4_index()];
        if !p3.flags().contains(EntryFlags::PRESENT) {
            return None;
        }

        let p2 = &mut p3.next_page_table_mut()[virt_addr.p3_index()];
        if !p2.flags().contains(EntryFlags::PRESENT) {
            return None;
        } else if p2.flags().contains(EntryFlags::HUGE_PAGE) {
            // 1 GiB
            let page_off = virt_addr.to_inner() & 0o7777777777;
            return Some((p2.phys_addr().offset(page_off), p2));
        }

        let p1 = &mut p2.next_page_table_mut()[virt_addr.p2_index()];
        if !p1.flags().contains(EntryFlags::PRESENT) {
            return None;
        } else if p1.flags().contains(EntryFlags::HUGE_PAGE) {
            // 2 MiB
            let page_off = virt_addr.to_inner() & 0o7777777;
            return Some((p1.phys_addr().offset(page_off), p1));
        }

        let p0 = &mut p1.next_page_table_mut()[virt_addr.p1_index()];
        if !p0.flags().contains(EntryFlags::PRESENT) {
            return None;
        } else {
            // 4KiB
            let page_off = virt_addr.to_inner() & 0o7777;
            return Some((p0.phys_addr().offset(page_off), p0));
        }
    }
}

impl Index<usize> for Table {
    type Output = Entry;

    fn index(&self, index: usize) -> &Entry {
        &self.entries[index]
    }
}

impl IndexMut<usize> for Table {
    fn index_mut(&mut self, index: usize) -> &mut Entry {
        &mut self.entries[index]
    }
}

#[inline]
fn tlb_flush(addr: VirtualAddress) {
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) addr.to_inner(), options(nostack, preserves_flags));
    }
}
