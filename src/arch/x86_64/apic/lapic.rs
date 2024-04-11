use log::trace;

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

            // set Task Priority Register to 0 so that it allows all external interrupts
            lapic.write_reg(0x80, 0);

            // set spurious vector and APIC software enable flag
            lapic.write_reg(0xf0, 0xff | 0x100);

            lapic.init_timer();

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

    unsafe fn init_timer(&self) {
        const DIVIDER: u16 = 0x3e0;
        const INIT_COUNT: u16 = 0x380;
        const CUR_COUNT: u16 = 0x390;
        const TIMER_LVT: u16 = 0x320;

        // set divider to 16
        // divider = 2 ** (i+1) for i = 0..6
        // divider = 1 if i = 7
        self.write_reg(DIVIDER, 0b11);

        // TODO: start HPET for calibration

        // initial count
        self.write_reg(INIT_COUNT, u32::MAX);

        // TODO: wait till you get the HPET interrupt

        // mask interrupts
        self.write_reg(TIMER_LVT, 1 << 16);


        // calculate APIC timer ticks during this time
        let cur_count = self.read_reg(CUR_COUNT);
        let ticks_occurred = u32::MAX - cur_count;

        // TODO calculate frequency and store it for further use
        
        // unset interrupt mask, set mode to periodic, set interrupt to 32 (IRQ0)
        self.write_reg(TIMER_LVT, 1 << 17 | 32);
        self.write_reg(DIVIDER, 0b11);
        self.write_reg(INIT_COUNT, ticks_occurred);
    }
}

// struct ApicTimer {

// }

// impl ApicTimer {
//     fn new() -> Self {
//         Self {}
//     }

//     unsafe fn init(&mut self) {
//         // APIC timer always running (skips need for calibration)
//         let CpuidResult { eax, .. } = core::arch::x86_64::__cpuid(0x6);
//         trace!("NO NEED FOR CALIBRATION: {:#b}", eax);
//         assert!(eax & 0b100 != 0);

//         // TODO: initialise LAPIC timer
//         // https://lore.kernel.org/all/20190417052810.3052-1-drake@endlessm.com/t/
//         // let CpuidResult { eax, ebx, ecx, edx } = core::arch::x86_64::__cpuid(0x15);
//         // trace!("0x15 leaf: eax: {}, ebx: {}, ecx: {}, edx: {}", eax, ebx, ecx, edx);

//         self.reg
//     }

// }