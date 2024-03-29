use super::pic;
use core::{arch::asm, ptr::addr_of};

const MSR_APIC_REG_BASE: u32 = 0x1b;
const ENABLE_APIC: u64 = 1 << 11;
const BASE_ADDR: u64 = !0b1111_1111_1111;
const OFFSET1: u64 = 0x300;
const OFFSET2: u64 = 0x310;

extern "C" {
    #[link_name = "_ap_start_location"]
    static AP_START: u8;
}

pub(super) fn ap_init() {
    unsafe {
        pic::disable();

        let msr_apic_reg_base = rdmsr(MSR_APIC_REG_BASE);
        crate::println!("APIC_BASE: {:#x}", msr_apic_reg_base);
        wrmsr(MSR_APIC_REG_BASE, msr_apic_reg_base | ENABLE_APIC);
    }

    // let bsp = (msr_apic_reg_base & (1 << 8)) >> 8 == 1;
    // crate::println!("BSP: {}", bsp);
    // let enable = (msr_apic_reg_base & (1 << 11)) >> 11 == 1;
    // crate::println!("ENABLE: {}", enable);
    // let base = msr_apic_reg_base & !0b1111_1111_1111;
    // crate::println!("BASE: {:#x}", base);
    unsafe {
        crate::println!("ap_start: {:#x?}", addr_of!(AP_START) as usize);
        let msr_apic_reg_base = rdmsr(MSR_APIC_REG_BASE);
        let base = msr_apic_reg_base & BASE_ADDR;
        crate::println!("base: {:#x?}", base);

        // let part1 = BASE_ADDR + OFFSET1;
        // let part2 = BASE_ADDR + OFFSET2;
        let part1: u64 = 0xFEE00300;
        let part2: u64 = 0xFEE00310;

        let init_data1 = 0b0000_0000_0000_0000_0100_0101_0000_0000;
        let init_data2 = 0b0000_0001_0000_0000_0000_0000_0000_0000;

        crate::println!("sending INIT to proc 1");
        crate::println!("INIT, write part2: {:#x?} @ {:#x?}", init_data2, part2 as *mut u32);
        crate::println!("INIT, write part1: {:#x?} @ {:#x?}", init_data1, part1 as *mut u32);
        core::intrinsics::volatile_store(part2 as *mut u32, init_data2);
        core::intrinsics::volatile_store(part1 as *mut u32, init_data1);

        while core::intrinsics::volatile_load(part1 as *mut u32) & (1 << 12) != 0 {
            core::hint::spin_loop();
        }
        crate::println!("Sent INIT to proc 1");
        crate::println!("After INIT, part2: {:#x?}", core::intrinsics::volatile_load(part2 as *mut u32));
        crate::println!("After INIT, part1: {:#x?}", core::intrinsics::volatile_load(part1 as *mut u32));
        
        // let mut i = 0;
        // while i < 1000000 {
        //     i += 1;
        //     core::hint::spin_loop();
        // }

        let sipi_data1 = 0b0000_0000_0000_0000_0100_0110_0000_0000;
        let sipi_data2 = 0b0000_0001_0000_0000_0000_0000_0000_0000;

        crate::println!("sending SIPI1 to proc 1");
        crate::println!("SIPI1, write part2: {:#x?} @ {:#x?}", sipi_data2, part2 as *mut u32);
        crate::println!("SIPI1, write part1: {:#x?} @ {:#x?}", sipi_data1, part1 as *mut u32);
        core::intrinsics::volatile_store(part2 as *mut u32, sipi_data2);
        core::intrinsics::volatile_store(part1 as *mut u32, sipi_data1);

        while core::intrinsics::volatile_load(part1 as *mut u32) & (1 << 12) != 0 {
            core::hint::spin_loop();
        }
        crate::println!("Sent SIPI1 to proc 1");
        crate::println!("After SIPI1, part2: {:#x?}", core::intrinsics::volatile_load(part2 as *mut u32));
        crate::println!("After SIPI1, part1: {:#x?}", core::intrinsics::volatile_load(part1 as *mut u32));
        
        // let mut i = 0;
        // while i < 1000000 {
        //     i += 1;
        //     core::hint::spin_loop();
        // }

        core::intrinsics::volatile_store(part2 as *mut u32, sipi_data2);
        core::intrinsics::volatile_store(part1 as *mut u32, sipi_data1);

        while core::intrinsics::volatile_load(part1 as *mut u32) & (1 << 12) != 0 {
            core::hint::spin_loop();
        }
        crate::println!("Sent SIPI2 to proc 1");
    }
}

pub unsafe fn rdmsr(msr: u32) -> u64 {
    let (high, low): (u32, u32);
    asm!("rdmsr", out("eax") low, out("edx") high, in("ecx") msr);
    ((high as u64) << 32) | (low as u64)
}

pub unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high);
}
