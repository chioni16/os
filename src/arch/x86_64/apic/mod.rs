use crate::arch::x86_64::{rdmsr, wrmsr};
use super::pic;
use core::arch::x86_64::CpuidResult;

pub(super) const MSR_APIC_REG_BASE: u32 = 0x1b;
pub(super) const APIC_ENABLE: u64 = 1 << 11;
pub(super) const LAPIC_PRESENT: u32 = 1 << 9;

pub(super) fn init_lapic() {
    unsafe {
        let CpuidResult { edx, .. } = core::arch::x86_64::__cpuid(1);
        assert!(edx & LAPIC_PRESENT != 0);

        pic::disable();

        let msr_apic_reg_base = rdmsr(MSR_APIC_REG_BASE);
        crate::println!("APIC_BASE: {:#x}", msr_apic_reg_base);
        wrmsr(MSR_APIC_REG_BASE, msr_apic_reg_base | APIC_ENABLE);
    }
}
