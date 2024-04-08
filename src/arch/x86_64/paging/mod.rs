pub mod entry;
mod page;
mod table;

use entry::EntryFlags;
use log::trace;
pub use table::{ActiveP4Table, P4Table};

use crate::{
    locks::SpinLock,
    mem::{PhysicalAddress, VirtualAddress},
};

pub static ACTIVE_PAGETABLE: SpinLock<ActiveP4Table> = ActiveP4Table::locked();

pub(super) fn init() {
    trace!("before new_table init");
    let mut new_page_table = P4Table::new();
    trace!("after new_table init");
    let virt_addr = VirtualAddress::new(0xf000000000);
    let phys_addr = PhysicalAddress::new(0xfffffff);
    trace!("before new_table map");
    new_page_table.map(
        virt_addr,
        phys_addr,
        EntryFlags::PRESENT | EntryFlags::WRITABLE,
    );
    trace!("after new_table map");

    trace!("before new_table unmapping");
    new_page_table.unmap(virt_addr);
    trace!("after new_table unmapping");
}

pub fn translate_using_current_page_table(virt_addr: VirtualAddress) -> Option<PhysicalAddress> {
    let mut guard = ACTIVE_PAGETABLE.lock();
    guard.translate(virt_addr)
}
