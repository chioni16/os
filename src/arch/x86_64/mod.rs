mod acpi;
mod apic;
mod gdt;
mod interrupts;
mod paging;
mod pci;
mod pic;
mod port;
mod process;
mod smp;
mod syscall;
mod timers;
mod userspace;
mod vga_buffer;

use crate::multiboot::MultibootInfo;
use log::info;

pub(crate) use interrupts::{disable_interrupts, enable_interrupts, is_int_enabled};
pub(crate) use paging::{entry::EntryFlags, get_cur_page_table_start, P4Table, ACTIVE_PAGETABLE};
pub(crate) use vga_buffer::_print;

pub(crate) fn init(multiboot_info: &MultibootInfo) {
    gdt::init();
    paging::init(multiboot_info);
    interrupts::init();
    pic::init();
    pci::init();

    let rsdt = acpi::find_rsdt().unwrap();
    let madt_entries = rsdt.find_madt().unwrap();
    info!("found madt: {:x?}", madt_entries);

    let hpet = timers::init(&rsdt);

    apic::init(&madt_entries, &hpet);
    smp::init_ap(&madt_entries);

    syscall::init();
    // userspace::run_userpace_code();
    process::init();
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
