pub mod allocator;
pub mod frame;

pub const PAGE_SIZE: u64 = 4096;
const HIGHER_HALF: u64 = 0xFFFF800000000000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
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

    // TODO: remove Option
    pub const unsafe fn to_virt(&self) -> Option<VirtualAddress> {
        Some(VirtualAddress::new(self.0 + HIGHER_HALF))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
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
    pub const fn as_const_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    // used when you suspect the data may not be aligned properly. example: multiboot structs
    // also used when we need a reference tied to a non-static lifetime. eg: page table
    pub const fn as_mut_ptr<T>(&self) -> *mut T {
        self.0 as *mut T
    }

    pub unsafe fn as_ref_static<T>(&self) -> &'static T {
        &*(self.0 as *const T)
    }

    pub unsafe fn as_mut_static<T>(&self) -> &'static mut T {
        &mut *(self.0 as *mut T)
    }
}

#[inline]
pub(crate) fn align_up(num: u64, align: u64) -> u64 {
    num.div_ceil(align) * align
}

#[inline]
pub(crate) fn align_down(num: u64, align: u64) -> u64 {
    (num / align) * align
}
