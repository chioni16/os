use crate::arch::translate_using_current_page_table;
use crate::locks::SpinLock;
use crate::mem::frame::Frame;
use crate::mem::{PhysicalAddress, VirtualAddress, PAGE_SIZE};
use crate::multiboot::{MemMapEntry, MemMapEntryType, MultibootInfo};
use log::{info, trace};

extern crate alloc;
use alloc::alloc::GlobalAlloc;

#[derive(Debug)]
pub struct BitMapAllocator(Option<BitMap>);

impl BitMapAllocator {
    const fn new() -> Self {
        Self(None)
    }

    pub const fn locked() -> SpinLock<Self> {
        let bitmap = Self::new();
        SpinLock::new(bitmap)
    }

    pub fn init(&mut self, multiboot_info: &MultibootInfo) {
        info!("called bitmap allocator init");
        if self.0.is_some() {
            panic!("Can be initialised only once");
        }

        let bitmap = BitMap::new(multiboot_info);
        self.0 = Some(bitmap);
    }
}

unsafe impl GlobalAlloc for SpinLock<BitMapAllocator> {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        trace!("requested alloc of {:#x} bytes", layout.size());
        let num_frames = layout.size().div_ceil(PAGE_SIZE as usize);
        if let Some(phys_addr) = self.lock().0.as_mut().unwrap().alloc(num_frames) {
            phys_addr.to_virt().unwrap().as_mut_ptr()
        } else {
            core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        trace!("requested dealloc of {:#x} bytes", layout.size());
        let num_frames = layout.size().div_ceil(PAGE_SIZE as usize);
        let ptr = translate_using_current_page_table(VirtualAddress::new(ptr as u64)).unwrap();
        self.lock().0.as_mut().unwrap().free(ptr, num_frames);
    }
}

/// used = 1
/// free = 0
#[derive(Debug)]
struct BitMap {
    inner: &'static mut [u8],
    // last_used_index: u64,
    /// RAM according to the Multiboot2 definitions
    used_ram_frames: u64,
    /// RAM according to the Multiboot2 definitions
    reserved_ram_frames: u64,
    /// RAM according to the Multiboot2 definitions
    ram_frames: u64,
}

impl BitMap {
    fn new(multiboot_info: &MultibootInfo) -> Self {
        let mem_regions = multiboot_info.multiboot_mem_tags().unwrap();
        let ram_regions = mem_regions
            .clone()
            .filter(|region| region.entry_type() == MemMapEntryType::Ram);
        let non_ram_regions = mem_regions
            .clone()
            .filter(|region| region.entry_type() != MemMapEntryType::Ram);

        // highest address occupied by the RAM regions
        // bitmap will cover upto this address
        let highest_ram_addr = ram_regions
            .clone()
            .map(|region| region.end())
            .max()
            .unwrap_or(PhysicalAddress::new(0));

        // amount of memory needed to store the bitmap
        let bitmap_size_in_bits = highest_ram_addr.to_inner().div_ceil(PAGE_SIZE); // num of frames to be mapped
        let bitmap_size_in_bytes = bitmap_size_in_bits.div_ceil(8);
        let bitmap_size_in_pages = bitmap_size_in_bytes.div_ceil(PAGE_SIZE);

        let elf_sections = multiboot_info
            .multiboot_elf_tags()
            .unwrap()
            .filter(|s| s.section_type() != 0);
        // last of the addresses occupied by kernel code
        let kernel_end = {
            let phys_addr = elf_sections.clone().map(|s| s.end()).max().unwrap();
            unsafe { translate_using_current_page_table(phys_addr).unwrap() }
        };
        let kernel_end_frame = Frame::containing_address(kernel_end);

        // choose the location where to place the bitmap
        // placed after the kernel code
        let bit_map_start_addr = ram_regions
            .clone()
            // possible locations that meet the criteria
            // 1. located after the kernel code
            // 2. has enough place to accommodate the bitmap
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
            // find the first area that satisfies the criteria
            .min()
            .unwrap();

        trace!("Non RAM regions");
        for region in non_ram_regions.clone() {
            trace!(
                "start: {:#x?}, end: {:#x?}",
                region.start().to_inner(),
                region.end().to_inner()
            );
        }
        trace!("highest ram address: {:#x?}", highest_ram_addr);
        trace!("kernel end: {:#x?}", kernel_end);
        trace!("kernel end frame: {:#x?}", kernel_end_frame);
        trace!("bitmap start address: {:#x?}", bit_map_start_addr);
        trace!(
            "bitmap size: {:#x} bytes ({:#x} pages)",
            bitmap_size_in_bytes,
            bitmap_size_in_pages
        );

        // works as intended when the sections are paage aligned
        // if not, can report more than the available number of frames
        // due to double counting at continuous non-page aligned region boundaries
        let num_frames_in_region = |region: MemMapEntry| region.length().div_ceil(PAGE_SIZE);
        let ram_frames = ram_regions.clone().map(num_frames_in_region).sum::<u64>();

        let inner = unsafe {
            let s = core::slice::from_raw_parts_mut(
                bit_map_start_addr.to_virt().unwrap().as_mut_ptr(),
                bitmap_size_in_bytes as usize,
            );
            s.fill(0);
            s
        };
        let mut bitmap = Self {
            inner,
            // last_used_index: 0,
            reserved_ram_frames: 0,
            used_ram_frames: 0,
            ram_frames,
        };

        // WARNING:
        // use inclusive ranges
        // last frame of the range should also be set / unset
        // using exclusive ranges doesn't do this

        for frame in 0..kernel_end_frame.number + 1 {
            bitmap.set(frame as usize);
            bitmap.reserved_ram_frames += 1;
        }
        trace!(
            "bitmap: reserved ram frames: from {:#x} to {:#x}",
            0,
            kernel_end_frame.number
        );

        // mark frames that host bitmap as unavailable
        let bitmap_frame = Frame::containing_address(bit_map_start_addr).number;
        for frame in bitmap_frame..bitmap_frame + bitmap_size_in_pages {
            bitmap.set(frame as usize);
            bitmap.reserved_ram_frames += 1;
        }
        trace!(
            "bitmap: bitmap location reserved ram frames: from {:#x} to {:#X}",
            bitmap_frame,
            bitmap_frame + bitmap_size_in_pages
        );

        // mark frames that belong to NON-RAM regions as unavailable
        non_ram_regions
            .clone()
            .filter(|region| region.start() < highest_ram_addr)
            .map(|region| {
                let start = Frame::containing_address(region.start()).number;
                let end = Frame::containing_address(region.end()).number;
                start..end + 1
            })
            .flatten()
            .for_each(|frame| {
                bitmap.set(frame as usize);
                // remove non-ram frames that were previously included
                if frame <= kernel_end_frame.number {
                    bitmap.reserved_ram_frames -= 1;
                }
            });

        info!(
            "Initialised bitmap: 
            reserved_ram_frames: {:#x},
            used_ram_frames: {:#x},
            ram_frames: {:#x},
        ",
            bitmap.reserved_ram_frames, bitmap.used_ram_frames, bitmap.ram_frames,
        );

        bitmap
    }

    fn get_free_frames(&self, num_frames: usize) -> Option<usize> {
        // TODO: optimisation
        // store the last value where the free frames were found
        // start searching for frames from this index
        // wrap around only if no free frames are found
        let mut cur_frames = 0;
        for index in 0..self.inner.len() * 8 {
            if !self.present(index) {
                cur_frames += 1;
            } else {
                cur_frames = 0;
            }

            if cur_frames == num_frames {
                return Some(index - (num_frames - 1));
            }
        }

        None
    }

    fn alloc(&mut self, num_frames: usize) -> Option<PhysicalAddress> {
        trace!("bitmap request to allocate {:#x} frames", num_frames);
        self.get_free_frames(num_frames).map(|start_index| {
            for index in start_index..start_index + num_frames {
                self.set(index);
            }
            self.used_ram_frames += num_frames as u64;
            trace!(
                "bitmap allocated frames {:#x}..{:#x}",
                start_index,
                start_index + num_frames
            );
            PhysicalAddress::new(start_index as u64 * PAGE_SIZE)
        })
    }

    fn free(&mut self, start: PhysicalAddress, num_frames: usize) {
        let start_index = Frame::containing_address(start).number as usize;
        for index in start_index..start_index + num_frames {
            self.reset(index);
        }
        self.used_ram_frames -= num_frames as u64;
        trace!(
            "bitmap freed frames {:#x}..{:#x}",
            start_index,
            start_index + num_frames
        );
    }

    #[inline]
    fn present(&self, frame: usize) -> bool {
        self.inner[frame / 8] & (1 << (frame % 8)) != 0
    }

    #[inline]
    fn set(&mut self, frame: usize) {
        self.inner[frame / 8] |= 1 << (frame % 8);
    }

    #[inline]
    fn reset(&mut self, frame: usize) {
        self.inner[frame / 8] &= !(1 << (frame % 8));
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
