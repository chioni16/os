use super::pic;
use core::arch::asm;

const MSR_APIC_REG_BASE: u32 = 0x1b;
const ENABLE_APIC: u64 = 1 << 11;
const BASE_ADDR: u64 = !0b1111_1111_1111;
const OFFSET1: u64 = 0x300;
const OFFSET2: u64 = 0x310;

pub(super) fn ap_init() {
    unsafe {
        pic::disable();

        let msr_apic_reg_base = rdmsr(MSR_APIC_REG_BASE);
        crate::println!("APIC_BASE: {:#b}", msr_apic_reg_base);
        wrmsr(MSR_APIC_REG_BASE, msr_apic_reg_base | ENABLE_APIC);
    }

    // let bsp = (msr_apic_reg_base & (1 << 8)) >> 8 == 1;
    // crate::println!("BSP: {}", bsp);
    // let enable = (msr_apic_reg_base & (1 << 11)) >> 11 == 1;
    // crate::println!("ENABLE: {}", enable);
    // let base = msr_apic_reg_base & !0b1111_1111_1111;
    // crate::println!("BASE: {:#x}", base);

    unsafe {
        let msr_apic_reg_base = rdmsr(MSR_APIC_REG_BASE);
        let base = msr_apic_reg_base & BASE_ADDR;

        let part1 = base + OFFSET1;
        let part2 = base + OFFSET2;

        let data1 = 0b0000_0000_0000_0000_0100_0101_0000_0000;
        let data2 = 0b0000_1000_0000_0000_0000_0000_0000_0000;
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
