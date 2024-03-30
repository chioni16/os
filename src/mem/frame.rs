use super::PAGE_SIZE;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    pub(super) number: u64,
}

impl Frame {
    pub(super) fn containing_address(address: u64) -> Frame {
        Frame {
            number: address / PAGE_SIZE,
        }
    }
}
