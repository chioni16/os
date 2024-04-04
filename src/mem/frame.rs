use super::{PhysicalAddress, PAGE_SIZE};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    pub number: u64,
}

impl Frame {
    pub fn containing_address(address: PhysicalAddress) -> Frame {
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
}
