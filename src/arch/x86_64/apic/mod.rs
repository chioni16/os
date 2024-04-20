mod ioapic;
pub(super) mod lapic;

use alloc::vec::Vec;

use super::{acpi, pic, timers::hpet::Hpet};
use crate::arch::x86_64::smp::is_bsp;
pub use lapic::send_eoi;

pub(super) fn init(madt_entries: &Vec<acpi::MadtEntry>, hpet: &Hpet) {
    unsafe {
        pic::disable();
    }

    lapic::Lapic::init(hpet);

    // IoApic is common to all the cores
    // So no need to do the same work multiple times in case of SMP
    if !is_bsp() {
        return;
    }

    for entry in madt_entries {
        match entry {
            // MadtEntry::LocalApic(lapic) => init_lapic(lapic),
            acpi::MadtEntry::IoApic(ioapic) => {
                ioapic::IoApic::new(*ioapic).init(madt_entries.iter().filter_map(|entry| {
                    if let acpi::MadtEntry::IoApicIntSourceOverride(or) = entry {
                        Some(*or)
                    } else {
                        None
                    }
                }))
            }
            _ => {}
        }
    }
}
