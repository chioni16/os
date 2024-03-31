use super::FrameAllocator;
use crate::mem::frame::Frame;
use crate::mem::PhysicalAddress;
use crate::multiboot::{MemMapEntry, MultibootIter};

pub struct AreaAllocator {
    next_free_frame: Frame,
    current_area: Option<MemMapEntry>,
    areas: MultibootIter<MemMapEntry>,
    kernel_start: Frame,
    kernel_end: Frame,
    multiboot_start: Frame,
    multiboot_end: Frame,
}

impl AreaAllocator {
    pub fn new(
        kernel_start: PhysicalAddress,
        kernel_end: PhysicalAddress,
        multiboot_start: PhysicalAddress,
        multiboot_end: PhysicalAddress,
        areas: MultibootIter<MemMapEntry>,
    ) -> Self {
        let mut allocator = AreaAllocator {
            next_free_frame: Frame::containing_address(0),
            current_area: None,
            areas,
            kernel_start: Frame::containing_address(kernel_start),
            kernel_end: Frame::containing_address(kernel_end),
            multiboot_start: Frame::containing_address(multiboot_start),
            multiboot_end: Frame::containing_address(multiboot_end),
        };
        allocator.choose_next_area();
        allocator
    }

    fn choose_next_area(&mut self) {
        self.current_area = self
            .areas
            .clone()
            .filter(|area| Frame::containing_address(area.end()) >= self.next_free_frame)
            .min_by_key(|area| area.start());

        if let Some(cur_area) = self.current_area {
            let start_frame = Frame::containing_address(cur_area.start());
            if self.next_free_frame < start_frame {
                self.next_free_frame = start_frame;
            }
        }
    }
}

impl FrameAllocator for AreaAllocator {
    fn allocate_frame(&mut self) -> Option<Frame> {
        while let Some(area) = self.current_area {
            let frame = Frame {
                number: self.next_free_frame.number,
            };
            let current_area_last_frame = Frame::containing_address(area.end());

            if frame > current_area_last_frame {
                self.choose_next_area();
                continue;
            }

            if frame >= self.kernel_start && frame <= self.kernel_end {
                self.next_free_frame = Frame {
                    number: self.kernel_end.number + 1,
                };
                continue;
            }

            if frame >= self.multiboot_start && frame <= self.multiboot_end {
                self.next_free_frame = Frame {
                    number: self.multiboot_end.number + 1,
                };
                continue;
            }

            self.next_free_frame.number += 1;
            return Some(frame);
        }

        None
    }

    fn deallocate_frame(&mut self, frame: Frame) {
        unimplemented!()
    }
}
