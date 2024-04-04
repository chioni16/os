use crate::mem::{VirtualAddress, PAGE_SIZE};

#[derive(Debug, Clone, Copy)]
pub struct Page {
    number: u64,
}

impl Page {
    pub fn containing_address(address: VirtualAddress) -> Page {
        // canonical addresses
        assert!(
            address < VirtualAddress::new(0x0000_8000_0000_0000)
                || address >= VirtualAddress::new(0xffff_8000_0000_0000),
            "invalid address: {:#x?}",
            address
        );
        Page {
            number: address.to_inner() / PAGE_SIZE,
        }
    }

    fn start_address(&self) -> VirtualAddress {
        VirtualAddress::new(self.number * PAGE_SIZE)
    }
}

impl VirtualAddress {
    pub(super) fn page_offset(&self) -> u64 {
        self.to_inner() % PAGE_SIZE
    }
    pub(super) fn p4_index(&self) -> usize {
        (self.to_inner() as usize >> 39) & 0o777
    }
    pub(super) fn p3_index(&self) -> usize {
        (self.to_inner() as usize >> 30) & 0o777
    }
    pub(super) fn p2_index(&self) -> usize {
        (self.to_inner() as usize >> 21) & 0o777
    }
    pub(super) fn p1_index(&self) -> usize {
        (self.to_inner() as usize >> 12) & 0o777
    }
}
