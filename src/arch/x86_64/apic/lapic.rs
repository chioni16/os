use crate::{
    arch::x86_64::{rdmsr, wrmsr},
    mem::{PhysicalAddress, VirtualAddress},
};
use core::arch::x86_64::CpuidResult;

pub const MSR_APIC_REG_BASE: u32 = 0x1b;
pub const APIC_ENABLE: u64 = 1 << 11;
pub const LAPIC_BASE_ADDR_MASK: u64 = !0b1111_1111_1111;

pub(super) const LAPIC_PRESENT: u32 = 1 << 9;

#[derive(Debug)]
pub(super) struct Lapic {
    // table: acpi::LocalApic,
    base: VirtualAddress,
}

// SAFETY: only to be called once the Local APIC is initialised
unsafe fn get_lapic() -> Lapic {
    let msr_apic_reg_base = rdmsr(MSR_APIC_REG_BASE);
    let base_phys = PhysicalAddress::new(msr_apic_reg_base & LAPIC_BASE_ADDR_MASK);
    let base = base_phys.to_virt().unwrap();
    Lapic { base }
}

// SAFETY: only to be called once the Local APIC is initialised
pub unsafe fn send_eoi() {
    let lapic = get_lapic();
    lapic.send_eoi();
}

impl Lapic {
    // pub(super) fn new(table: acpi::LocalApic) -> Self {
    pub(super) fn init() -> Self {
        unsafe {
            // ensure presence of LAPIC on this core
            let CpuidResult { edx, .. } = core::arch::x86_64::__cpuid(1);
            assert!(edx & LAPIC_PRESENT != 0);

            // set APIC global enable flag
            let msr_apic_reg_base = rdmsr(MSR_APIC_REG_BASE);
            wrmsr(MSR_APIC_REG_BASE, msr_apic_reg_base | APIC_ENABLE);

            crate::println!("APIC_BASE: {:#x}", msr_apic_reg_base);
            let base_phys = PhysicalAddress::new(msr_apic_reg_base & LAPIC_BASE_ADDR_MASK);
            let base = base_phys.to_virt().unwrap();

            let lapic = Self {
                // table,
                base,
            };

            // set spurious vector and APIC software enable flag
            lapic.write_reg(0xf0, 0xff | 0x100);

            lapic
        }
    }

    #[inline]
    fn read_reg(&self, offset: u16) -> u32 {
        unsafe {
            let addr = self.base.offset(offset as u64).as_const_ptr();
            core::intrinsics::volatile_load(addr)
        }
    }

    #[inline]
    fn write_reg(&self, offset: u16, val: u32) {
        unsafe {
            let addr = self.base.offset(offset as u64).as_mut_ptr();
            core::intrinsics::volatile_store(addr, val);
        }
    }

    #[inline]
    fn send_eoi(&self) {
        self.write_reg(0xb0, 0);
    }
}
