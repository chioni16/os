pub mod allocator;
pub mod frame;

pub const PAGE_SIZE: u64 = 4096;

pub type PhysicalAddress = u64;
pub type VirtualAddress = u64;
