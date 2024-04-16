use crate::mem::{frame::Frame, PhysicalAddress};
use crate::multiboot::{Elf64SectionFlags, Elf64SectionHeader};
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

impl EntryFlags {
    pub fn from_elf_section_flags(section: &Elf64SectionHeader) -> Self {
        let mut flags = Self::empty();

        if section.flags().contains(Elf64SectionFlags::SHF_ALLOC) {
            // section is loaded to memory
            flags = flags | Self::PRESENT;
        }
        if section.flags().contains(Elf64SectionFlags::SHF_WRITE) {
            flags = flags | Self::WRITABLE;
        }
        if !section.flags().contains(Elf64SectionFlags::SHF_EXECINSTR) {
            flags = flags | Self::NO_EXECUTE;
        }

        flags
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
            let phys_addr = self.phys_addr();
            Some(Frame::containing_address(phys_addr))
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
        PhysicalAddress::new(self.0 & PHYSADDR_MASK)
    }

    pub fn next_page_table(&self) -> Option<&Table> {
        if self.flags().contains(EntryFlags::PRESENT) {
            // SAFETY: `PRESENT` flag is set.
            // the `map` method ensures that the entry points to a valid page table
            let table = unsafe { &*self.phys_addr().to_virt().unwrap().as_const_ptr() };
            Some(table)
        } else {
            None
        }
    }

    pub fn next_page_table_mut(&mut self) -> Option<&mut Table> {
        if self.flags().contains(EntryFlags::PRESENT) {
            // SAFETY: `PRESENT` flag is set.
            // the `map` method ensures that the entry points to a valid page table
            let table = unsafe { &mut *self.phys_addr().to_virt().unwrap().as_mut_ptr() };
            Some(table)
        } else {
            None
        }
    }
}
