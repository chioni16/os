pub mod area_allocator;
pub mod bitmap_allocator;

use super::frame::Frame;

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);
}
