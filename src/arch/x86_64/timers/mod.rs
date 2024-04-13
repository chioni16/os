use log::info;

use crate::arch::x86_64::smp::is_bsp;

use super::acpi;

pub(crate) mod hpet;

pub(super) fn init(rsdt: &acpi::RsdtEntries) -> hpet::Hpet {
    let hpet = rsdt.find_hpet().unwrap();
    let acpi::AcpiSdtType::Hpet(hpet) = hpet.fields else {
        unreachable!()
    };
    info!("found hpet: {:x?}", hpet);
    let mut hpet = hpet::Hpet::new(hpet);

    // HPET is common to all the cores
    // So no need to do the same work multiple times in case of SMP
    if is_bsp() {
        // SAFETY: `init` function is called just once
        // run on BSP and skipped on other cores
        unsafe {
            hpet.init();
        }
    }

    hpet
}
