use bitflags::bitflags;
use core::marker::PhantomData;
use core::ptr;

use crate::mem::{PhysicalAddress, VirtualAddress};

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Elf64SectionHeader {
    sh_name: u32,
    sh_type: u32,
    sh_flags: u64,
    sh_addr: u64,
    sh_offset: u64,
    sh_size: u64,
    sh_link: u32,
    sh_info: u32,
    sh_addralign: u64,
    sh_entsize: u64,
}

bitflags! {
    pub struct Elf64SectionFlags: u64 {
        const SHF_WRITE      = 0x1;
        const SHF_ALLOC      = 0x2;
        const SHF_EXECINSTR  = 0x4;
        const SHF_MERGE      = 0x10;
        const SHF_STRINGS    = 0x20;
        const SHF_INFO_LINK  = 0x40;
        const SHF_LINK_ORDER = 0x80;
        const SHF_OS_NONCONFORMING = 0x100;
        const SHF_GROUP      = 0x200;
        const SHF_TLS        = 0x400;
        const SHF_MASKOS     = 0x0ff00000;
        const SHF_MASKPROC   = 0xf0000000;
    }
}

impl Elf64SectionHeader {
    pub fn section_type(&self) -> u32 {
        self.sh_type
    }

    pub fn start(&self) -> VirtualAddress {
        VirtualAddress::new(self.sh_addr)
    }

    pub fn size(&self) -> u64 {
        self.sh_size
    }

    // last address that belongs to the entry
    pub fn end(&self) -> VirtualAddress {
        self.start().offset(self.size() - 1)
    }

    pub fn flags(&self) -> Elf64SectionFlags {
        Elf64SectionFlags::from_bits_truncate(self.sh_flags)
    }

    pub fn contains(&self, addr: VirtualAddress) -> bool {
        self.start() <= addr && addr <= self.end()
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct MemMapEntry {
    base_addr: u64,
    length: u64,
    entry_type: u32,
    reserved: u32,
}

impl MemMapEntry {
    pub fn start(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.base_addr)
    }

    pub fn length(&self) -> u64 {
        self.length
    }

    // last address that belongs to the entry
    pub fn end(&self) -> PhysicalAddress {
        self.start().offset(self.length - 1)
    }

    pub fn entry_type(&self) -> MemMapEntryType {
        self.entry_type.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemMapEntryType {
    Ram,
    Acpi,
    Preserved,
    DefectiveRam,
    Other(u32),
}

impl From<u32> for MemMapEntryType {
    fn from(value: u32) -> Self {
        match value {
            1 => Self::Ram,
            3 => Self::Acpi,
            4 => Self::Preserved,
            5 => Self::DefectiveRam,
            o => Self::Other(o),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MultibootIter<T> {
    start: VirtualAddress,
    cur: u32,
    total_size: u32,
    _marker: PhantomData<T>,
}

impl<T> Iterator for MultibootIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur >= self.total_size {
            return None;
        }

        // SAFETY:
        // we are only dereferencing the addresses that fall within the limits of what the multiboot protocol returns
        // start to start + size
        let entry = unsafe {
            let addr = self.start.offset(self.cur as u64).as_const_ptr();
            ptr::read_unaligned(addr)
        };

        self.cur += core::mem::size_of::<T>() as u32;

        Some(entry)
    }
}

pub struct MultibootInfo {
    base: PhysicalAddress,
    size: u32,
}

impl MultibootInfo {
    pub fn new(addr: u64) -> Self {
        let base = PhysicalAddress::new(addr);
        Self {
            base,
            // SAFETY:
            // we are only dereferencing the addresses that fall within the limits of what the multiboot protocol returns
            // start to start + size
            size: unsafe { ptr::read_unaligned(base.to_virt().unwrap().as_const_ptr()) },
        }
    }

    pub fn start(&self) -> PhysicalAddress {
        self.base
    }

    pub fn size(&self) -> u64 {
        self.size as u64
    }

    // last address that belongs to MultibootInfo
    pub fn end(&self) -> PhysicalAddress {
        self.base.offset(self.size() - 1)
    }

    fn find_tags_of_type(&self, tag_type: u32) -> Option<(VirtualAddress, u32)> {
        let addr = unsafe { self.base.to_virt().unwrap() };
        // SAFETY:
        // we are only dereferencing the addresses that fall within the limits of what the multiboot protocol returns
        // start to start + size
        unsafe {
            let total_length = ptr::read_unaligned(addr.as_const_ptr());
            assert_eq!(ptr::read_unaligned(addr.offset(4).as_const_ptr::<u32>()), 0);
            let mut cur_len = 8u32;
            while cur_len < total_length {
                let cur_type =
                    ptr::read_unaligned(addr.offset(cur_len as u64).as_const_ptr::<u32>());
                let cur_size =
                    ptr::read_unaligned(addr.offset(cur_len as u64 + 4).as_const_ptr::<u32>());

                if cur_type == tag_type {
                    let start_addr = addr.offset(cur_len as u64);
                    return Some((start_addr, cur_size));
                }

                if cur_type == 0 {
                    cur_len += 8;
                    continue;
                }

                cur_len += cur_size;

                // Padding to maintain 8 byte alignment at the beginning of a new tag
                let diff = (8 - cur_len as i64 + (cur_len as i64 / 8) * 8) % 8;
                assert!(diff >= 0);
                cur_len += diff as u32;
            }

            assert_eq!(cur_len, total_length);
        }

        None
    }

    pub fn multiboot_elf_tags(&self) -> Option<MultibootIter<Elf64SectionHeader>> {
        self.find_tags_of_type(9).map(|(start_addr, total_size)| {
            // SAFETY:
            // we are only dereferencing the addresses that fall within the limits of what the multiboot protocol returns
            // start to start + size
            unsafe {
                // consider padding
                let entry_size = ptr::read_unaligned(start_addr.offset(12).as_const_ptr::<u16>());
                assert_eq!(
                    entry_size as usize,
                    core::mem::size_of::<Elf64SectionHeader>()
                );
            }

            MultibootIter {
                start: start_addr,
                cur: 20,
                total_size,
                _marker: PhantomData,
            }
        })
    }

    // assumes presence of only one memory tags entry
    pub fn multiboot_mem_tags(&self) -> Option<MultibootIter<MemMapEntry>> {
        self.find_tags_of_type(6).map(|(start_addr, total_size)| {
            // SAFETY:
            // we are only dereferencing the addresses that fall within the limits of what the multiboot protocol returns
            // start to start + size
            unsafe {
                let entry_size = ptr::read_unaligned(start_addr.offset(8).as_const_ptr::<u32>());
                assert_eq!(entry_size as usize, core::mem::size_of::<MemMapEntry>());
                // supports only entry_version 0
                assert_eq!(
                    ptr::read_unaligned(start_addr.offset(12).as_const_ptr::<u32>()),
                    0
                );
            }

            MultibootIter {
                start: start_addr,
                cur: 16,
                total_size,
                _marker: PhantomData,
            }
        })
    }
}
