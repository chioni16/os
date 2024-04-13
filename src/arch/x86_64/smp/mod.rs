use crate::{
    arch::x86_64::{
        acpi::{AcpiSdt, AcpiSdtType, MadtEntry},
        apic::{APIC_ENABLE, MSR_APIC_REG_BASE},
        paging::Mmio,
        rdmsr,
    },
    mem::PhysicalAddress,
};
use core::{
    arch::x86_64::{CpuidResult, __cpuid},
    ptr::addr_of,
};

const IS_BSP: u64 = 1 << 8;
const BASE_ADDR: u64 = !0b1111_1111_1111;
const OFFSET1: u64 = 0x300;
const OFFSET2: u64 = 0x310;

extern "C" {
    #[link_name = "_ap_start_location"]
    static AP_START: u8;
}

pub(super) fn init_ap(madt: &AcpiSdt) {
    unsafe {
        let msr_apic_reg_base = rdmsr(MSR_APIC_REG_BASE);
        crate::println!("APIC_BASE: {:#x}", msr_apic_reg_base);

        // this code is intended to run only on BSP
        let bsp = (msr_apic_reg_base & IS_BSP) != 1;
        assert!(bsp);

        let is_apic_enabled = (msr_apic_reg_base & APIC_ENABLE) != 1;
        assert!(is_apic_enabled);

        let CpuidResult { ebx, .. } = __cpuid(1);
        let bspid = (ebx >> 24) as u8;
        // let enable = (msr_apic_reg_base & (1 << 11)) >> 11 == 1;
        // crate::println!("ENABLE: {}", enable);
        // let base = msr_apic_reg_base & !0b1111_1111_1111;
        // crate::println!("BASE: {:#x}", base);
        crate::println!("ap_start: {:#x?}", addr_of!(AP_START) as usize);
        let msr_apic_reg_base = rdmsr(MSR_APIC_REG_BASE);
        let base = msr_apic_reg_base & BASE_ADDR;
        crate::println!("base: {:#x?}", base);

        // TODO: MMIO module and mapping the physical addresses properly with strongly uncacheable properties
        let base = PhysicalAddress::new(base);
        let mmio = Mmio::new(base, base.offset(0x400));
        let part1: *mut u32 = base.offset(OFFSET1).to_virt().unwrap().as_mut_ptr();
        let part2: *mut u32 = base.offset(OFFSET2).to_virt().unwrap().as_mut_ptr();

        const INIT_DATA1: u32 = 0b0000_0000_0000_0000_0100_0101_0000_0000;
        const INIT_DATA2: u32 = 0b0000_0000_0000_0000_0000_0000_0000_0000;

        const SIPI_DATA1: u32 = 0b0000_0000_0000_0000_0100_0110_0000_0000;
        const SIPI_DATA2: u32 = 0b0000_0000_0000_0000_0000_0000_0000_0000;

        let AcpiSdtType::Madt { entries, .. } = &madt.fields else {
            unreachable!()
        };
        for entry in entries {
            if let MadtEntry::LocalApic(lapic) = entry {
                if lapic.aid == bspid {
                    continue;
                }
                // INIT
                crate::println!("sending INIT to LAPIC {}", lapic.aid);
                let init_data2 = INIT_DATA2 | (lapic.aid as u32) << 24;
                crate::println!("INIT, write part2: {:#x?} @ {:#x?}", init_data2, part2);
                crate::println!("INIT, write part1: {:#x?} @ {:#x?}", INIT_DATA1, part1);
                core::intrinsics::volatile_store(part2, init_data2);
                core::intrinsics::volatile_store(part1, INIT_DATA1);
                while core::intrinsics::volatile_load(part1) & (1 << 12) != 0 {
                    core::hint::spin_loop();
                }
                crate::println!("Sent INIT to LAPIC {}", lapic.aid);
                crate::println!(
                    "After INIT, part2: {:#x?}",
                    core::intrinsics::volatile_load(part2)
                );
                crate::println!(
                    "After INIT, part1: {:#x?}",
                    core::intrinsics::volatile_load(part1)
                );

                // TODO: wait for some time before sending SIPI1

                // SIPI1
                crate::println!("sending SIPI1 to LAPIC {}", lapic.aid);
                let sipi_data2 = SIPI_DATA2 | (lapic.aid as u32) << 24;
                crate::println!("SIPI1, write part2: {:#x?} @ {:#x?}", sipi_data2, part2);
                crate::println!("SIPI1, write part1: {:#x?} @ {:#x?}", SIPI_DATA1, part1);
                core::intrinsics::volatile_store(part2, sipi_data2);
                core::intrinsics::volatile_store(part1, SIPI_DATA1);
                while core::intrinsics::volatile_load(part1) & (1 << 12) != 0 {
                    core::hint::spin_loop();
                }
                crate::println!("Sent SIPI1 to proc 1");
                crate::println!(
                    "After SIPI1, part2: {:#x?}",
                    core::intrinsics::volatile_load(part2)
                );
                crate::println!(
                    "After SIPI1, part1: {:#x?}",
                    core::intrinsics::volatile_load(part1)
                );

                // TODO: SIPI2?
            }
        }
        drop(mmio);
    }
}
