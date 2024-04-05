pub mod allocator;
pub mod frame;

pub const PAGE_SIZE: u64 = 4096;
// 3 GiB
const HIGHER_HALF: u64 = 0xC0000000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(u64);

impl PhysicalAddress {
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    pub fn to_inner(self) -> u64 {
        self.0
    }

    pub const fn offset(&self, offset: u64) -> Self {
        Self::new(self.0 + offset)
    }

    pub const unsafe fn to_virt(&self) -> Option<VirtualAddress> {
        // 4 GiB
        // size of available ram
        if self.0 < 0x100000000 {
            Some(VirtualAddress::new(self.0 + HIGHER_HALF))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualAddress(u64);

impl VirtualAddress {
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    pub fn to_inner(self) -> u64 {
        self.0
    }

    pub fn offset(&self, offset: u64) -> Self {
        Self::new(self.0 + offset)
    }

    // used when you suspect the data may not be aligned properly. example: multiboot structs
    // also used when we need a reference tied to a non-static lifetime. eg: page table
    pub const unsafe fn as_const_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    // used when you suspect the data may not be aligned properly. example: multiboot structs
    // also used when we need a reference tied to a non-static lifetime. eg: page table
    pub const unsafe fn as_mut_ptr<T>(&self) -> *mut T {
        self.0 as *mut T
    }

    pub unsafe fn as_ref_static<T>(&self) -> &'static T {
        &*(self.0 as *const T)
    }

    pub unsafe fn as_mut_static<T>(&self) -> &'static mut T {
        &mut *(self.0 as *mut T)
    }
}
