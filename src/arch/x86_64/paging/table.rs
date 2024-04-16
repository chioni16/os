use log::trace;

use super::entry::{Entry, EntryFlags, PAGE_ENTRY_SIZE};
use super::ACTIVE_PAGETABLE;
use crate::locks::SpinLock;
use crate::mem::frame::Frame;
use crate::mem::{PhysicalAddress, VirtualAddress, PAGE_SIZE};
use crate::{HEAP_ALLOCATOR, HIGHER_HALF};
use core::ops::{Index, IndexMut};
use core::ptr::addr_of;

use crate::mem::allocator::FrameAllocator;

const PAGE_ENTRY_COUNT: u64 = PAGE_SIZE / PAGE_ENTRY_SIZE;

#[derive(Debug)]
#[repr(align(4096))]
pub(super) struct Table {
    entries: [Entry; PAGE_ENTRY_COUNT as usize],
}

impl Table {
    fn zero() -> Self {
        Self {
            entries: [Entry::zero(); PAGE_ENTRY_COUNT as usize],
        }
    }
}

impl Index<usize> for Table {
    type Output = Entry;

    fn index(&self, index: usize) -> &Entry {
        &self.entries[index]
    }
}

impl IndexMut<usize> for Table {
    fn index_mut(&mut self, index: usize) -> &mut Entry {
        &mut self.entries[index]
    }
}

// WARNING: Keep this unique (no Clone / Copy)
// Don't expose the inner details (addr) to the outside world
#[derive(Debug, PartialEq, Eq)]
pub struct P4Table {
    addr: PhysicalAddress,
}

impl P4Table {
    // SAFETY: ensure that the given physical address points to a valid and non-deallocated P4 page table
    pub unsafe fn from_addr(addr: PhysicalAddress) -> Self {
        Self { addr }
    }

    pub fn new() -> Self {
        let new_frame = Self::alloc_page_table();
        Self {
            addr: new_frame.start_address(),
        }
    }

    fn is_active(&self) -> bool {
        let guard = ACTIVE_PAGETABLE.lock();
        *self == *guard.as_ref()
    }

    fn alloc_page_table() -> Frame {
        // use the heap allocator allocate frame method
        HEAP_ALLOCATOR.lock().allocate_frame().unwrap()
    }

    // SAFETY: ensure that the frame is a valid allocated frame from the allocator
    unsafe fn dealloc_page_table(table: &mut Table) {
        // use the heap allocator free frame method
        let virt_addr = table as *const Table as u64;
        let phys_addr = PhysicalAddress::new(virt_addr - addr_of!(HIGHER_HALF) as u64);
        HEAP_ALLOCATOR
            .lock()
            .deallocate_frame(Frame::containing_address(phys_addr));
    }

    pub fn map_huge_1GiB(
        &mut self,
        virt_addr: VirtualAddress,
        phys_addr: PhysicalAddress,
        flags_to_set: EntryFlags,
    ) {
        assert!(flags_to_set.contains(EntryFlags::HUGE_PAGE));

        // SAFETY: all of this code should run from the kernel space
        // So, even though the page tables are changed to kernel's page table on entering kernel space from userspace,
        // as the kernel memory is mapped to the higher half of every userspace program, the translation by adding a fixed offset should yield valid addresses
        // these upper half addresses are only accessible by kernel. ignoring meltdown/spectre ;)
        let p4 = unsafe { &mut *self.addr.to_virt().unwrap().as_mut_ptr::<Table>() };

        // entry in p4 table corresponding to the p3 table
        let p4_entry = &mut p4[virt_addr.p4_index()];
        if !p4_entry.flags().contains(EntryFlags::PRESENT) {
            let new_frame = Self::alloc_page_table();
            p4_entry.set(new_frame, EntryFlags::PRESENT);
        }
        if flags_to_set.contains(EntryFlags::WRITABLE) {
            p4_entry.set_flags(EntryFlags::WRITABLE);
        }
        if flags_to_set.contains(EntryFlags::USER_ACCESSIBLE) {
            p4_entry.set_flags(EntryFlags::USER_ACCESSIBLE);
        }

        let p3 = p4_entry.next_page_table_mut().unwrap();
        // entry in p3 table corresponding to the corresponding leaf frame
        let p3_entry = &mut p3[virt_addr.p3_index()];
        p3_entry.set(Frame::containing_address(phys_addr), flags_to_set);
    }

    pub fn map_huge_2MiB(
        &mut self,
        virt_addr: VirtualAddress,
        phys_addr: PhysicalAddress,
        flags_to_set: EntryFlags,
    ) {
        assert!(flags_to_set.contains(EntryFlags::HUGE_PAGE));

        // SAFETY: all of this code should run from the kernel space
        // So, even though the page tables are changed to kernel's page table on entering kernel space from userspace,
        // as the kernel memory is mapped to the higher half of every userspace program, the translation by adding a fixed offset should yield valid addresses
        // these upper half addresses are only accessible by kernel. ignoring meltdown/spectre ;)
        let p4 = unsafe { &mut *self.addr.to_virt().unwrap().as_mut_ptr::<Table>() };

        // entry in p4 table corresponding to the p3 table
        let p4_entry = &mut p4[virt_addr.p4_index()];
        if !p4_entry.flags().contains(EntryFlags::PRESENT) {
            let new_frame = Self::alloc_page_table();
            p4_entry.set(new_frame, EntryFlags::PRESENT);
        }
        if flags_to_set.contains(EntryFlags::WRITABLE) {
            p4_entry.set_flags(EntryFlags::WRITABLE);
        }
        if flags_to_set.contains(EntryFlags::USER_ACCESSIBLE) {
            p4_entry.set_flags(EntryFlags::USER_ACCESSIBLE);
        }

        let p3 = p4_entry.next_page_table_mut().unwrap();
        // entry in p3 table corresponding to the p2 table
        let p3_entry = &mut p3[virt_addr.p3_index()];
        if !p3_entry.flags().contains(EntryFlags::PRESENT) {
            let new_frame = Self::alloc_page_table();
            p3_entry.set(new_frame, EntryFlags::PRESENT);
        }
        if flags_to_set.contains(EntryFlags::WRITABLE) {
            p3_entry.set_flags(EntryFlags::WRITABLE);
        }
        if flags_to_set.contains(EntryFlags::USER_ACCESSIBLE) {
            p3_entry.set_flags(EntryFlags::USER_ACCESSIBLE);
        }

        let p2 = p3_entry.next_page_table_mut().unwrap();
        // entry in p2 table corresponding to the leaf frame
        let p2_entry = &mut p2[virt_addr.p2_index()];
        p2_entry.set(Frame::containing_address(phys_addr), flags_to_set);
    }

    pub fn map_4KiB(
        &mut self,
        virt_addr: VirtualAddress,
        phys_addr: PhysicalAddress,
        flags_to_set: EntryFlags,
    ) {
        assert!(!flags_to_set.contains(EntryFlags::HUGE_PAGE));

        // SAFETY: all of this code should run from the kernel space
        // So, even though the page tables are changed to kernel's page table on entering kernel space from userspace,
        // as the kernel memory is mapped to the higher half of every userspace program, the translation by adding a fixed offset should yield valid addresses
        // these upper half addresses are only accessible by kernel. ignoring meltdown/spectre ;)
        let p4 = unsafe { &mut *self.addr.to_virt().unwrap().as_mut_ptr::<Table>() };

        // entry in p4 table corresponding to the p3 table
        let p4_entry = &mut p4[virt_addr.p4_index()];
        if !p4_entry.flags().contains(EntryFlags::PRESENT) {
            let new_frame = Self::alloc_page_table();
            p4_entry.set(new_frame, EntryFlags::PRESENT);
        }
        if flags_to_set.contains(EntryFlags::WRITABLE) {
            p4_entry.set_flags(EntryFlags::WRITABLE);
        }
        if flags_to_set.contains(EntryFlags::USER_ACCESSIBLE) {
            p4_entry.set_flags(EntryFlags::USER_ACCESSIBLE);
        }

        let p3 = p4_entry.next_page_table_mut().unwrap();
        // entry in p3 table corresponding to the p2 table
        let p3_entry = &mut p3[virt_addr.p3_index()];
        if !p3_entry.flags().contains(EntryFlags::PRESENT) {
            let new_frame = Self::alloc_page_table();
            p3_entry.set(new_frame, EntryFlags::PRESENT);
        }
        if flags_to_set.contains(EntryFlags::WRITABLE) {
            p3_entry.set_flags(EntryFlags::WRITABLE);
        }
        if flags_to_set.contains(EntryFlags::USER_ACCESSIBLE) {
            p3_entry.set_flags(EntryFlags::USER_ACCESSIBLE);
        }

        let p2 = p3_entry.next_page_table_mut().unwrap();
        // entry in p2 table corresponding to the p1 table
        let p2_entry = &mut p2[virt_addr.p2_index()];
        if !p2_entry.flags().contains(EntryFlags::PRESENT) {
            let new_frame = Self::alloc_page_table();
            p2_entry.set(new_frame, EntryFlags::PRESENT);
        }
        if flags_to_set.contains(EntryFlags::WRITABLE) {
            p2_entry.set_flags(EntryFlags::WRITABLE);
        }
        if flags_to_set.contains(EntryFlags::USER_ACCESSIBLE) {
            p2_entry.set_flags(EntryFlags::USER_ACCESSIBLE);
        }

        let p1 = p2_entry.next_page_table_mut().unwrap();
        // entry in p1 table corresponding to the leaf frame
        let p1_entry = &mut p1[virt_addr.p1_index()];
        p1_entry.set(Frame::containing_address(phys_addr), flags_to_set);
    }

    // TODO: &mut self as traverse requires it - find a workaround without having to duplicate traverse code
    pub fn translate(&mut self, virt_addr: VirtualAddress) -> Option<PhysicalAddress> {
        self.traverse(virt_addr).map(|(phys_addr, _)| phys_addr)
    }

    pub fn unmap(&mut self, virt_addr: VirtualAddress) -> bool {
        if let Some((_, entry)) = self.traverse(virt_addr) {
            entry.unset_flags(EntryFlags::PRESENT);
            // TODO: deallocating page table frames necessary here?
            // If you change this remember to modify the drop implementation to avoid double free
            // IMPORTANT: Better, just set the physical address to 0 when you free a page table
            // but will this cause an issue in the future, when you handle demand paging?
            // At the moment, `traverse` method only returns leaf table entries
            // i.e, P1 / Huge table entries

            // Bigger TODO:
            // remove entries from the higher level tables if the current table has no active entry
            // here we may have to deallocate frames

            true
        } else {
            false
        }
    }

    fn traverse(&mut self, virt_addr: VirtualAddress) -> Option<(PhysicalAddress, &mut Entry)> {
        // SAFETY: all of this code should run from the kernel space
        // So, even though the page tables are changed to kernel's page table on entering kernel space from userspace,
        // as the kernel memory is mapped to the higher half of every userspace program, the translation by adding a fixed offset should yield valid addresses
        // these upper half addresses are only accessible by kernel. ignoring meltdown/spectre ;)
        let p4 = unsafe { &mut *self.addr.to_virt().unwrap().as_mut_ptr::<Table>() };

        // entry in p4 table corresponding to the p3 table
        let p4_entry = &mut p4[virt_addr.p4_index()];
        if !p4_entry.flags().contains(EntryFlags::PRESENT) {
            return None;
        }

        let p3 = p4_entry.next_page_table_mut().unwrap();
        // entry in p3 table corresponding to the p2 table
        let p3_entry = &mut p3[virt_addr.p3_index()];
        if !p3_entry.flags().contains(EntryFlags::PRESENT) {
            return None;
        } else if p3_entry.flags().contains(EntryFlags::HUGE_PAGE) {
            // 1 GiB
            let page_off = virt_addr.to_inner() & 0o7777777777;
            return Some((p3_entry.phys_addr().offset(page_off), p3_entry));
        }

        let p2 = p3_entry.next_page_table_mut().unwrap();
        // entry in p2 table corresponding to the p1 table
        let p2_entry = &mut p2[virt_addr.p2_index()];
        if !p2_entry.flags().contains(EntryFlags::PRESENT) {
            return None;
        } else if p2_entry.flags().contains(EntryFlags::HUGE_PAGE) {
            // 2 MiB
            let page_off = virt_addr.to_inner() & 0o7777777;
            return Some((p2_entry.phys_addr().offset(page_off), p2_entry));
        }

        let p1 = p2_entry.next_page_table_mut().unwrap();
        // entry in p1 table corresponding to the leaf frame
        let p1_entry = &mut p1[virt_addr.p1_index()];
        if !p1_entry.flags().contains(EntryFlags::PRESENT) {
            return None;
        } else {
            // 4KiB
            let page_off = virt_addr.to_inner() & 0o7777;
            return Some((p1_entry.phys_addr().offset(page_off), p1_entry));
        }
    }
}

impl Drop for P4Table {
    fn drop(&mut self) {
        trace!("dropping page table");
        // SAFETY: all of this code should run from the kernel space
        // So, even though the page tables are changed to kernel's page table on entering kernel space from userspace,
        // as the kernel memory is mapped to the higher half of every userspace program, the translation by adding a fixed offset should yield valid addresses
        // these upper half addresses are only accessible by kernel. ignoring meltdown/spectre ;)
        let p4 = unsafe { &mut *self.addr.to_virt().unwrap().as_mut_ptr::<Table>() };

        // SAFETY: the p4 table OWNS all the entries and the tables that the entries point to.
        // This applies to the rest of the tables in the page table hierarchy.
        // Only the frames used to store the page tables are deallocated.
        // The leaf frames (that DON'T belong to the page tables) are not deallocated.

        for p4_entry in &mut p4.entries {
            // TODO: what should I look for to skip the entry?
            // depends on the `unmap` implementation
            // but for now this should suffice

            // If the entry has the huge page flag set, it means that it points to "actual" frames and not to another page table frame
            // In this case, don't free the actual frame

            if p4_entry.phys_addr().to_inner() == 0
                || p4_entry.flags().contains(EntryFlags::HUGE_PAGE)
            {
                continue;
            }

            let p3 = p4_entry.next_page_table_mut().unwrap();
            for p3_entry in &mut p3.entries {
                if p3_entry.phys_addr().to_inner() == 0
                    || p3_entry.flags().contains(EntryFlags::HUGE_PAGE)
                {
                    continue;
                }

                let p2 = p3_entry.next_page_table_mut().unwrap();
                for p2_entry in &mut p2.entries {
                    if p2_entry.phys_addr().to_inner() == 0
                        || p2_entry.flags().contains(EntryFlags::HUGE_PAGE)
                    {
                        continue;
                    }

                    let p1 = p2_entry.next_page_table_mut().unwrap();

                    // SAFETY: Addresses stored in a page table should point to a valid, non-deallocated page table
                    // This is because the frames are deallocated only when the entire P4Table is dropped
                    unsafe { Self::dealloc_page_table(p1) };
                }
                unsafe { Self::dealloc_page_table(p2) };
            }
            unsafe { Self::dealloc_page_table(p3) };
        }
        unsafe { Self::dealloc_page_table(p4) };
    }
}

// WARNING:
// Stricter restrictions compared to `P4Table`
// There should ever be only one copy of this struct
// i.e, the global static variable protected by a lock.
#[derive(Debug)]
pub struct ActiveP4Table {
    inner: Option<P4Table>,
}

impl ActiveP4Table {
    // There should ever be only one copy of this struct
    // So, don't make this method available from outside the module
    pub(super) const fn locked() -> SpinLock<Self> {
        let p4 = Self { inner: None };
        SpinLock::new(p4)
    }

    pub fn init(&mut self) {
        let p4 = P4Table {
            // SAFETY: the `init` function is called in the rust code
            // Paging is enabled by the time `rust_start` is called
            addr: unsafe { Self::get_current_page_table_addr() },
        };
        self.inner = Some(p4);
    }

    pub fn switch(&mut self, new_p4: P4Table) -> P4Table {
        trace!("old page table: {:#x?}", self);
        trace!("new page table: {:#x?}", new_p4);

        // SAFETY: paging is enabled by the time we get here
        // and the `ActiveP4Table` always holds the same value as the CR3 register
        // This is guaranteed partly because any changes to `ActiveP4Table` occurs through a lock
        unsafe {
            // TLB flushed automatically in this case
            // no need for `invlpg` instruction
            core::arch::asm!(
                "mov cr3, {new_phys}",
                new_phys = in(reg) new_p4.addr.to_inner(),
            );
        }

        core::mem::replace(self.as_mut(), new_p4)
    }

    pub fn translate(&mut self, virt_addr: VirtualAddress) -> Option<PhysicalAddress> {
        self.as_mut().translate(virt_addr)
    }

    pub fn map_huge_1GiB(
        &mut self,
        virt_addr: VirtualAddress,
        phys_addr: PhysicalAddress,
        flags_to_set: EntryFlags,
    ) {
        self.as_mut()
            .map_huge_1GiB(virt_addr, phys_addr, flags_to_set);
    }

    pub fn map_huge_2MiB(
        &mut self,
        virt_addr: VirtualAddress,
        phys_addr: PhysicalAddress,
        flags_to_set: EntryFlags,
    ) {
        self.as_mut()
            .map_huge_2MiB(virt_addr, phys_addr, flags_to_set);
    }

    pub fn map_4KiB(
        &mut self,
        virt_addr: VirtualAddress,
        phys_addr: PhysicalAddress,
        flags_to_set: EntryFlags,
    ) {
        self.as_mut().map_4KiB(virt_addr, phys_addr, flags_to_set);
    }

    pub fn unmap(&mut self, virt_addr: VirtualAddress) -> bool {
        let found = self.as_mut().unmap(virt_addr);

        // SAFETY: unmapping occurs from the ActiveP4Table
        // which is always in sync with the CR3 register
        unsafe {
            tlb_flush(virt_addr);
        }

        found
    }

    fn as_ref(&self) -> &P4Table {
        self.inner.as_ref().unwrap()
    }

    fn as_mut(&mut self) -> &mut P4Table {
        self.inner.as_mut().unwrap()
    }

    // SAFETY: paging should be enabled
    unsafe fn get_current_page_table_addr() -> PhysicalAddress {
        let p4: u64;
        core::arch::asm!("mov rax, cr3", out("rax") p4);
        PhysicalAddress::new(p4)
    }
}

// SAFETY: ensure that the `VirtualAddress` belongs to the `ActiveP4Table`
#[inline]
unsafe fn tlb_flush(addr: VirtualAddress) {
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) addr.to_inner(), options(nostack, preserves_flags));
    }
}

// SAFETY: paging should be enabled
#[inline]
pub(super) unsafe fn tlb_flush_all() {
    unsafe {
        core::arch::asm!(
            "mov {tmp}, cr3",
            "mov cr3, {tmp}",
            tmp = out(reg) _,
            options(nostack)
        );
    }
}
