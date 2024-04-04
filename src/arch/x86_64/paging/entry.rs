use crate::mem::{frame::Frame, PhysicalAddress, PAGE_SIZE};
use bitflags::bitflags;

use super::table::Table;

const PHYSADDR_MASK: u64 = 0x000fffff_fffff000;
pub const PAGE_ENTRY_SIZE: u64 = core::mem::size_of::<Entry>() as u64;

bitflags! {
    pub struct EntryFlags: u64 {
        const PRESENT =         1 << 0;
        const WRITABLE =        1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const WRITE_THROUGH =   1 << 3;
        const NO_CACHE =        1 << 4;
        const ACCESSED =        1 << 5;
        const DIRTY =           1 << 6;
        const HUGE_PAGE =       1 << 7;
        const GLOBAL =          1 << 8;
        const NO_EXECUTE =      1 << 63;
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Entry(u64);

impl Entry {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    pub fn set_zero(&mut self) {
        self.0 = 0;
    }

    pub fn flags(&self) -> EntryFlags {
        EntryFlags::from_bits_truncate(self.0)
    }

    pub fn pointed_frame(&self) -> Option<Frame> {
        if self.flags().contains(EntryFlags::PRESENT) {
            Some(Frame::containing_address(PhysicalAddress::new(
                self.0 & PHYSADDR_MASK,
            )))
        } else {
            None
        }
    }

    pub fn set(&mut self, frame: Frame, flags: EntryFlags) {
        assert!(frame.start_address().to_inner() & !PHYSADDR_MASK == 0);
        self.0 = frame.start_address().to_inner() | flags.bits();
    }

    // set flags in addition to those that are already set
    pub fn set_flags(&mut self, flags: EntryFlags) {
        self.0 = self.0 | flags.bits();
    }

    pub fn unset_flags(&mut self, flags: EntryFlags) {
        self.0 = self.0 & !flags.bits();
    }

    pub fn phys_addr(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.0 & (((1 << 40) - 1) * PAGE_SIZE))
    }

    pub fn next_page_table(&self) -> &'static Table {
        unsafe { self.phys_addr().to_virt().unwrap().as_ref_static() }
    }

    pub fn next_page_table_mut(&mut self) -> &'static mut Table {
        unsafe { self.phys_addr().to_virt().unwrap().as_mut_static() }
    }
}
