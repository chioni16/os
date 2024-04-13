use super::{PhysicalAddress, PAGE_SIZE};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Frame {
    pub(crate) number: u64,
}

impl Frame {
    pub(crate) fn containing_address(address: PhysicalAddress) -> Frame {
        Frame {
            number: address.to_inner() / PAGE_SIZE,
        }
    }

    pub fn start_address(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.number * PAGE_SIZE)
    }

    fn clone(&self) -> Frame {
        Frame {
            number: self.number,
        }
    }

    pub(crate) fn range_inclusive(start: &Frame, end: &Frame) -> FrameIter {
        FrameIter {
            start: start.clone(),
            end: end.clone(),
        }
    }
}

pub(crate) struct FrameIter {
    start: Frame,
    end: Frame,
}

impl Iterator for FrameIter {
    type Item = Frame;

    fn next(&mut self) -> Option<Frame> {
        if self.start <= self.end {
            let frame = self.start.clone();
            self.start.number += 1;
            Some(frame)
        } else {
            None
        }
    }
}
