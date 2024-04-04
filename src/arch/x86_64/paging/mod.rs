use crate::mem::{PhysicalAddress, VirtualAddress};

extern crate alloc;
use alloc::boxed::Box;

pub mod entry;
mod page;
mod table;

pub use table::Table;

pub unsafe fn get_current_page_table() -> &'static mut Table {
    let mut p4: u64;
    core::arch::asm!("mov rax, cr3", out("rax") p4);

    PhysicalAddress::new(p4).to_virt().unwrap().as_mut_static()
}

// SAFETY: only pass a p4 table
pub unsafe fn switch_current_page_table(new_p4: Box<Table>) -> PhysicalAddress {
    let old_p4 = get_current_page_table();

    let new_p4_virtual = VirtualAddress::new(&*new_p4 as *const _ as u64);
    let new_p4_physical = old_p4.translate(new_p4_virtual).unwrap();

    let old_p4_physical;
    core::arch::asm!(
        "mov {old_phys}, cr3",
        "mov cr3, {new_phys}",
        new_phys = in(reg) new_p4_physical.to_inner(),
        old_phys = out(reg) old_p4_physical,
    );

    PhysicalAddress::new(old_p4_physical)
}

pub unsafe fn translate_using_current_page_table(
    virt_addr: VirtualAddress,
) -> Option<PhysicalAddress> {
    let pt = get_current_page_table();
    pt.translate(virt_addr)
}
