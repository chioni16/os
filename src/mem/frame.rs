use super::{PhysicalAddress, PAGE_SIZE};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    pub number: u64,
}

impl Frame {
    pub fn containing_address(address: PhysicalAddress) -> Frame {
        Frame {
            number: address / PAGE_SIZE,
        }
    }

    pub fn start_address(&self) -> PhysicalAddress {
        self.number * PAGE_SIZE
    }
}
