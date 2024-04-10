mod acpi;
mod apic;
mod interrupts;
mod paging;
mod pci;
mod pic;
mod port;
mod smp;
mod timers;
mod vga_buffer;

pub(crate) use interrupts::{disable_interrupts, enable_interrupts, is_int_enabled};
pub(crate) use paging::{translate_using_current_page_table, ACTIVE_PAGETABLE};
pub(crate) use vga_buffer::_print;

pub(crate) fn init() {
    paging::init();
    interrupts::init();
    pic::init();
    pci::init();

    let rsdt = acpi::find_rsdt();
    let acpi::AcpiSdtType::Rsdt(rsdt) = rsdt.unwrap().fields else {
        unreachable!()
    };
    let madt = rsdt.find_madt().unwrap();
    crate::println!("found madt: {:x?}", madt.fields);

    apic::init(&madt);
    smp::init_ap(&madt);
}

unsafe fn rdmsr(msr: u32) -> u64 {
    let (high, low): (u32, u32);
    core::arch::asm!("rdmsr", out("eax") low, out("edx") high, in("ecx") msr);
    ((high as u64) << 32) | (low as u64)
}

unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    core::arch::asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high);
}
