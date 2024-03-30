use core::ptr;

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

impl Elf64SectionHeader {
    pub fn section_type(&self) -> u32 {
        self.sh_type
    }

    pub fn base_addr(&self) -> u64 {
        self.sh_addr
    }

    pub fn size(&self) -> u64 {
        self.sh_size
    }

    pub fn flags(&self) -> u64 {
        self.sh_flags
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
    pub fn start(&self) -> u64 {
        self.base_addr
    }

    pub fn length(&self) -> u64 {
        self.length
    }

    pub fn end(&self) -> u64 {
        self.base_addr + self.length - 1
    }

    pub fn entry_type(&self) -> MemMapEntryType {
        self.entry_type().try_into().unwrap()
    }
}

pub enum MemMapEntryType {
    Ram = 1,
    Acpi = 3,
    Preserved = 4,
    DefectiveRam = 5,
}

impl TryFrom<u32> for MemMapEntryType {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let mmet = match value {
            1 => Self::Ram,
            3 => Self::Acpi,
            4 => Self::Preserved,
            5 => Self::DefectiveRam,
            _ => return Err(()),
        };

        Ok(mmet)
    }
}

#[derive(Debug, Clone)]
pub struct MultibootIter<T> {
    start: *const T,
    cur: u32,
    total_size: u32,
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
            let addr = self.start.byte_add(self.cur as usize);
            ptr::read_unaligned(addr)
        };

        self.cur += core::mem::size_of::<T>() as u32;

        Some(entry)
    }
}

pub struct MultibootInfo {
    base: u64,
    size: u32,
}

impl MultibootInfo {
    pub fn new(addr: u64) -> Self {
        Self {
            base: addr,
            // SAFETY:
            // we are only dereferencing the addresses that fall within the limits of what the multiboot protocol returns
            // start to start + size
            size: unsafe { ptr::read_unaligned(addr as *const u32) },
        }
    }

    pub fn start(&self) -> u64 {
        self.base
    }

    pub fn size(&self) -> u64 {
        self.size as u64
    }

    pub fn end(&self) -> u64 {
        self.base + self.size as u64
    }

    fn find_tags_of_type(&self, tag_type: u32) -> Option<(u64, u32)> {
        let addr = self.base;
        // SAFETY:
        // we are only dereferencing the addresses that fall within the limits of what the multiboot protocol returns
        // start to start + size
        unsafe {
            let total_length = ptr::read_unaligned(addr as *const u32);
            assert_eq!(ptr::read_unaligned((addr + 4) as *const u32), 0);

            let mut cur_len = 8u32;
            while cur_len < total_length {
                let cur_type = ptr::read_unaligned((addr + cur_len as u64) as *const u32);
                let cur_size = ptr::read_unaligned((addr + cur_len as u64 + 4) as *const u32);

                if cur_type == tag_type {
                    let start_addr = addr + cur_len as u64;
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
                let entry_size = ptr::read_unaligned((start_addr + 12) as *const u16);
                assert_eq!(
                    entry_size as usize,
                    core::mem::size_of::<Elf64SectionHeader>()
                );
            }

            MultibootIter {
                start: start_addr as *const Elf64SectionHeader,
                cur: 20,
                total_size,
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
                let entry_size = ptr::read_unaligned((start_addr + 8) as *const u32);
                assert_eq!(entry_size as usize, core::mem::size_of::<MemMapEntry>());
                // supports only entry_version 0
                assert_eq!(ptr::read_unaligned((start_addr + 12) as *const u32), 0);
            }

            MultibootIter {
                start: start_addr as *const MemMapEntry,
                cur: 16,
                total_size,
            }
        })
    }
}
