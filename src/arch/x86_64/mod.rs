mod acpi;
mod interrupts;
mod paging;
mod pci;
mod pic;
mod port;
mod vga_buffer;

pub(crate) use paging::translate_using_current_page_table;
pub(crate) use vga_buffer::_print;

pub(crate) fn init() {
    interrupts::init();
    pic::init();
    pci::init();

    let rsdt = acpi::find_rsdt();
    let acpi::AcpiSdtType::Rsdt(rsdt) = rsdt.unwrap().fields else {
        unreachable!()
    };

    let madt = rsdt.find_madt().unwrap();
    use crate::{
        arch::x86_64::paging::entry::EntryFlags,
        mem::{PhysicalAddress, VirtualAddress},
    };
    let mut new_page_table = unsafe { paging::Table::new() };
    unsafe {
        let virt_addr = VirtualAddress::new(0xf000000000);
        let phys_addr = PhysicalAddress::new(0xfffffff);
        new_page_table.map(
            virt_addr,
            phys_addr,
            EntryFlags::PRESENT | EntryFlags::WRITABLE,
        );

        new_page_table.unmap(virt_addr);
    }
}
