mod acpi;
mod interrupts;
mod paging;
mod pci;
mod pic;
mod port;
mod vga_buffer;

pub(crate) fn init() {
    interrupts::init();
    pic::init();
    pci::init();

    let rsdt = acpi::find_rsdt();
    crate::println!("found rsdt: {:x?}", rsdt);
    let acpi::AcpiSdtType::Rsdt(rsdt) = rsdt.unwrap().fields else {
        unreachable!()
    };

    let madt = rsdt.find_madt().unwrap();
    crate::println!("found madt: {:x?}", madt.fields);
}

pub(crate) use vga_buffer::_print;
