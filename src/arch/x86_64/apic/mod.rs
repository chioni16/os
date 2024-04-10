mod ioapic;
pub(super) mod lapic;

use super::{
    acpi::{self, AcpiSdt, AcpiSdtType, MadtEntry},
    pic,
};
use crate::arch::x86_64::smp::is_bsp;
pub use lapic::send_eoi;

pub(super) fn init(madt: &AcpiSdt) {
    unsafe {
        pic::disable();
    }

    lapic::Lapic::init();

    // IoApic is common to all the cores
    // So no need to do the same work multiple times in case of SMP
    if !is_bsp() {
        return;
    }

    let AcpiSdtType::Madt { entries, .. } = &madt.fields else {
        unreachable!()
    };

    for entry in entries {
        match entry {
            // MadtEntry::LocalApic(lapic) => init_lapic(lapic),
            MadtEntry::IoApic(ioapic) => {
                ioapic::IoApic::new(*ioapic).init(entries.iter().filter_map(|entry| {
                    if let MadtEntry::IoApicIntSourceOverride(or) = entry {
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
