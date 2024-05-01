use crate::mem::{PhysicalAddress, VirtualAddress, PAGE_SIZE};

use super::{EntryFlags, P4Table};
use bitflags::bitflags;
use log::info;

bitflags! {
    pub struct Elf64SegmentFlags: u32 {
        // executable
        const PF_X = 0x1;
        // writable
        const PF_W = 0x2;
        // readable
        const PF_R = 0x4;
    }
}

impl From<Elf64SegmentFlags> for EntryFlags {
    fn from(sflags: Elf64SegmentFlags) -> Self {
        let mut flags = Self::empty();

        // we are mapping user accessible pages here
        flags |= Self::USER_ACCESSIBLE;


        // all LOAD segments should be readable?

        if sflags.contains(Elf64SegmentFlags::PF_R) {
            flags |= Self::PRESENT;
        }

        if sflags.contains(Elf64SegmentFlags::PF_W) {
            flags |= Self::WRITABLE;
        }

        if !sflags.contains(Elf64SegmentFlags::PF_X) {
            flags |= Self::NO_EXECUTE;
        }

        flags
    }
}

pub(super) fn load_elf(start: PhysicalAddress, size: usize) -> (VirtualAddress, P4Table) {
    info!("[load_elf] start: {:#x?}, size: {:#x?}", start, size);

    let bytes = unsafe {
        let start = start.to_virt().unwrap();
        core::slice::from_raw_parts(start.to_inner() as *const u8, size as usize)
    };

    let binary = goblin::elf::Elf::parse(bytes).unwrap();

    // magic
    assert_eq!(binary.header.e_ident[..4], [0x7f, 0x45, 0x4c, 0x46]);
    // class (0x2 - 64 bit)
    assert_eq!(binary.header.e_ident[4], 0x2);
    // endian (0x1 - little)
    assert_eq!(binary.header.e_ident[5], 0x1);
    // version - 0x1
    assert_eq!(binary.header.e_ident[6], 0x1);
    // OS ABI - 0x0
    assert_eq!(binary.header.e_ident[7], 0x0);
    // machine - x86-64
    assert_eq!(binary.header.e_machine, 0x3e);
    // version - 0x1
    assert_eq!(binary.header.e_version, 0x1);

    let entry = VirtualAddress::new(binary.entry);

    let mut page_table = unsafe { P4Table::with_kernel_mapped_to_higher_half() };

    for segment in binary.program_headers {
        match segment.p_type {
            // NULL
            0 => {}
            // LOAD
            1 => {
                // page aligned
                assert_eq!(segment.p_align % 0x1000, 0);
                // don't want to deal with the cases where they are different
                assert_eq!(segment.p_vaddr, segment.p_paddr);

                // TODO: BSS section not supported yet
                assert_eq!(segment.p_filesz, segment.p_memsz);

                let num_pages = segment.p_memsz.div_ceil(PAGE_SIZE);
                let seg_start_virt = VirtualAddress::new(segment.p_vaddr);
                let seg_start_mem = start.offset(segment.p_offset);
                for i in 0..num_pages {
                    page_table.map_4KiB(
                        seg_start_virt.offset(i * PAGE_SIZE),
                        seg_start_mem.offset(i * PAGE_SIZE),
                        Elf64SegmentFlags::from_bits(segment.p_flags)
                            .unwrap()
                            .into(),
                    );
                }
            }
            // TODO: support DYNAMIC segments for relocation
            _ => unimplemented!(),
        }
    }

    // TODO: handle relocations
    match binary.header.e_type {
        // REL
        1 => {}
        // EXEC
        2 => {}
        _ => unimplemented!(),
    }

    (entry, page_table)
}
