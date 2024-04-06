use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::translate_using_current_page_table;
use crate::locks::SpinLock;
use crate::mem::frame::Frame;
use crate::mem::{PhysicalAddress, VirtualAddress, PAGE_SIZE};
use crate::multiboot::{Elf64SectionFlags, MemMapEntry, MemMapEntryType, MultibootInfo};

static INIT: AtomicBool = AtomicBool::new(false);

extern crate alloc;
use alloc::alloc::GlobalAlloc;

#[derive(Debug)]
pub struct BitMapAllocator {
    bitmap: Option<BitMap>,
    num_pages: usize,
    // last_used_index: u64,
    // usable_pages: u64,
    // used_pages: u64,
    // reserved_pages: u64,
}

impl BitMapAllocator {
    const fn new() -> Self {
        Self {
            bitmap: None,
            num_pages: 0,
            // last_used_index: 0,
            // usable_pages: 0,
            // used_pages: 0,
            // reserved_pages: 0,
        }
    }

    pub const fn locked() -> SpinLock<Self> {
        let bitmap = Self::new();
        SpinLock::new(bitmap)
    }

    pub fn init(&mut self, multiboot_info: &MultibootInfo) {
        if INIT.load(Ordering::Relaxed) {
            panic!("Can only be initialised once");
        }

        INIT.store(true, Ordering::Relaxed);

        let mem_regions = multiboot_info.multiboot_mem_tags().unwrap();
        let ram_regions = mem_regions
            .clone()
            .filter(|region| region.entry_type() == MemMapEntryType::Ram);
        let non_ram_regions = mem_regions
            .clone()
            .filter(|region| region.entry_type() != MemMapEntryType::Ram);

        // works as intended when the sections are paage aligned
        // if not, can report more than the available number of pages due to double counting at continuous non-page aligned region boundaries
        let num_pages_in_region = |region: MemMapEntry| region.length().div_ceil(PAGE_SIZE);
        let usable_pages = ram_regions.clone().map(num_pages_in_region).sum::<u64>();
        let reserved_pages = non_ram_regions
            .clone()
            .map(num_pages_in_region)
            .sum::<u64>();

        let highest_addr = ram_regions
            .clone()
            .map(|region| region.end())
            .max()
            .unwrap_or(PhysicalAddress::new(0));

        // memory needed to store the bitmap
        let bitmap_size_in_bits = highest_addr.to_inner().div_ceil(PAGE_SIZE); // num of pages to be mapped
        let bitmap_size_in_bytes = bitmap_size_in_bits.div_ceil(8);
        let bitmap_size_in_pages = align_up(bitmap_size_in_bytes, PAGE_SIZE);

        crate::println!("highest address: {:#x?}", highest_addr);
        crate::println!("bitmap size: {:x} bytes", bitmap_size_in_bytes);

        let elf_sections = multiboot_info
            .multiboot_elf_tags()
            .unwrap()
            .filter(|s| s.section_type() != 0);
        let kernel_end = elf_sections.clone().map(|s| s.end()).max().unwrap();
        let kernel_end = unsafe { translate_using_current_page_table(kernel_end).unwrap() };

        let bit_map_start_addr = ram_regions
            .clone()
            .filter_map(|region| {
                if region.start() > kernel_end {
                    let proposed_start = align_up(region.start().to_inner(), PAGE_SIZE);
                    if region.end().to_inner() - proposed_start >= bitmap_size_in_bytes {
                        Some(PhysicalAddress::new(proposed_start))
                    } else {
                        None
                    }
                } else if region.end() > kernel_end {
                    let proposed_start = align_up(kernel_end.to_inner(), PAGE_SIZE);
                    if region.end().to_inner() - proposed_start >= bitmap_size_in_bytes {
                        Some(PhysicalAddress::new(proposed_start))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .min()
            // .and_then(|addr| unsafe { addr.to_virt() })
            .unwrap();

        let mut bitmap = BitMap::new(unsafe {
            core::slice::from_raw_parts_mut(
                bit_map_start_addr.to_virt().unwrap().as_mut_ptr(),
                bitmap_size_in_bytes as usize,
            )
        });
        non_ram_regions
            .clone()
            .filter(|region| region.start() < kernel_end)
            .map(|region| {
                let start = Frame::containing_address(region.start()).number;
                let end = Frame::containing_address(region.end()).number;
                start..end
            })
            .flatten()
            .for_each(|frame| bitmap.set(frame as usize));

        elf_sections
            .clone()
            .filter(|section| section.flags().contains(Elf64SectionFlags::SHF_ALLOC))
            .map(|section| unsafe {
                let start = Frame::containing_address(
                    translate_using_current_page_table(section.start()).unwrap(),
                )
                .number;
                let end = Frame::containing_address(
                    translate_using_current_page_table(section.end()).unwrap(),
                )
                .number;
                start..end
            })
            .flatten()
            .for_each(|frame| bitmap.set(frame as usize));

        let bitmap_frame = Frame::containing_address(bit_map_start_addr).number;
        for frame in bitmap_frame..bitmap_frame + bitmap_size_in_pages {
            bitmap.set(frame as usize);
        }

        self.bitmap = Some(bitmap);
        self.num_pages = bitmap_size_in_bits as usize;
    }

    fn get_free_frames(&mut self, num_frames: usize) -> Option<usize> {
        let bitmap = self.bitmap.as_mut().unwrap();

        let mut cur_pages = 0;
        for cur_index in 0..self.num_pages {
            if !bitmap.present(cur_index) {
                cur_pages += 1;
            } else {
                cur_pages = 0;
            }

            if cur_pages == num_frames {
                return Some(cur_index - (num_frames - 1));
            }
        }

        None
    }

    pub fn _alloc(&mut self, num_frames: usize) -> Option<PhysicalAddress> {
        self.get_free_frames(num_frames).map(|start_index| {
            let bitmap = self.bitmap.as_mut().unwrap();
            for index in start_index..start_index + num_frames {
                bitmap.set(index);
            }
            let start_frame = Frame {
                number: start_index as u64,
            };
            start_frame.start_address()
        })
    }

    pub fn _free(&mut self, start: PhysicalAddress, num_frames: usize) {
        let bitmap = self.bitmap.as_mut().unwrap();
        let start_index = Frame::containing_address(start).number as usize;
        for index in start_index..start_index + num_frames {
            bitmap.reset(index);
        }
    }
}

unsafe impl GlobalAlloc for SpinLock<BitMapAllocator> {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let num_frames = layout.size();
        if let Some(phys_addr) = self.lock()._alloc(num_frames) {
            phys_addr.to_virt().unwrap().as_mut_ptr()
        } else {
            core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let num_frames = layout.size();
        let ptr = translate_using_current_page_table(VirtualAddress::new(ptr as u64)).unwrap();
        self.lock()._free(ptr, num_frames);
    }
}

/// used = 1
/// free = 0
#[derive(Debug)]
struct BitMap(&'static mut [u8]);

impl BitMap {
    fn new(s: &'static mut [u8]) -> Self {
        s.fill(0);
        Self(s)
    }

    #[inline]
    fn present(&self, bit: usize) -> bool {
        self.0[bit / 8] & (1 << (bit % 8)) != 0
    }

    #[inline]
    fn set(&mut self, bit: usize) {
        self.0[bit / 8] |= 1 << (bit % 8);
    }

    #[inline]
    fn reset(&mut self, bit: usize) {
        self.0[bit / 8] &= !(1 << (bit % 8));
    }
}

#[inline]
fn align_up(num: u64, align: u64) -> u64 {
    num.div_ceil(align) * align
}

#[inline]
fn align_down(num: u64, align: u64) -> u64 {
    (num / align) * align
}
