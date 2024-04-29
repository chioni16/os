use log::trace;

use crate::{
    arch::x86_64::{paging::mmio, rdmsr, timers::hpet::Hpet, wrmsr},
    mem::{PhysicalAddress, VirtualAddress},
};
use core::{
    arch::x86_64::CpuidResult,
    sync::atomic::{AtomicU64, AtomicU8, Ordering},
};

pub const MSR_APIC_REG_BASE: u32 = 0x1b;
pub const APIC_ENABLE: u64 = 1 << 11;
pub const LAPIC_BASE_ADDR_MASK: u64 = !0b1111_1111_1111;

pub(super) const LAPIC_PRESENT: u32 = 1 << 9;

// WARNING: Assuming that APIC timers on all LAPICs have same frequency
static APIC_TIMER_FREQ: AtomicU64 = AtomicU64::new(0);
static APIC_TIMER_DIVIDER: AtomicU8 = AtomicU8::new(0);

#[derive(Debug)]
pub(in super::super) struct Lapic {
    // table: acpi::LocalApic,
    base: VirtualAddress,
    // timer_freq_considering_divider: Option<u32>,
}

// SAFETY: only to be called once the Local APIC is initialised
pub(in super::super) unsafe fn get_lapic() -> Lapic {
    let msr_apic_reg_base = rdmsr(MSR_APIC_REG_BASE);
    let base_phys = PhysicalAddress::new(msr_apic_reg_base & LAPIC_BASE_ADDR_MASK);
    let base = base_phys.to_virt().unwrap();
    Lapic {
        base,
        // timer_freq_considering_divider: None
    }
}

// SAFETY: only to be called once the Local APIC is initialised
pub unsafe fn send_eoi() {
    let lapic = get_lapic();
    lapic.send_eoi();
}

impl Lapic {
    // pub(super) fn new(table: acpi::LocalApic) -> Self {
    pub(super) fn init(hpet: &Hpet) -> Self {
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

            // intel sdm vol 3 Table 11-1
            mmio::map(base_phys, base_phys.offset(0x3ff));

            let mut lapic = Self {
                // table,
                base,
                // timer_freq_considering_divider: None,
            };

            // set Task Priority Register to 0 so that it allows all external interrupts
            lapic.write_reg(0x80, 0);

            // set spurious vector and APIC software enable flag
            lapic.write_reg(0xf0, 0xff | 0x100);

            lapic.init_timer(hpet);

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

    // APIC TIMER

    const DIVIDER: u16 = 0x3e0;
    const INIT_COUNT: u16 = 0x380;
    const CURRENT_COUNT: u16 = 0x390;
    const TIMER_LVT: u16 = 0x320;
    const TIMER_INTERRUPT: u16 = 16;

    unsafe fn init_timer(&mut self, hpet: &Hpet) {
        let divider = 128;
        self.set_timer_divider(divider);
        // mask interrupts
        self.mask_timer_interrupts(true);

        // start HPET for calibration
        hpet.write_main_counter(0);
        // 1 sec
        let counter = hpet.ns_to_counter(10u64.pow(9));

        // APIC timer initial count
        self.set_timer_initial_count(u32::MAX);

        // wait till you get the HPET interrupt
        while hpet.read_main_counter() < counter {
            core::hint::spin_loop();
        }

        // WARNING: The method assumes that there were no wrap arounds in the Apic timer
        // during this time.
        // For this to be true, try to keep the divisor value high (64 / 128) for calibration

        // calculate APIC timer ticks during this time
        let ticks_occurred_in_1_sec = u32::MAX - self.get_timer_current_count();
        trace!(
            "APIC timer frequency {} for divider {}",
            ticks_occurred_in_1_sec,
            divider
        );

        // calculate frequency and store it for further use
        let new_divider = 64;
        APIC_TIMER_FREQ.store(
            ticks_occurred_in_1_sec as u64 * divider as u64,
            Ordering::Relaxed,
        );
        self.set_timer_divider(new_divider);
        APIC_TIMER_DIVIDER.store(new_divider, Ordering::Relaxed);

        // // cast to u64 to avoid overflows when calculating
        // let net_freq = (ticks_occurred_in_1_sec as u64 * divider as u64) / new_divider as u64;
        // self.timer_freq_considering_divider = Some(net_freq as u32);

        // unset interrupt mask, set mode to oneshot, set interrupt to 32 (IRQ0)
        self.set_timer_interrupt_vector(32); // IRQ0
        self.set_timer_mode(ApicTimerMode::Oneshot);

        // self.set_timer_initial_count(2 * ticks_occurred_in_1_sec);
        // self.set_timer_initial_count_in_ns(2 * 10u32.pow(9));
        self.set_timer_initial_count_in_ns(0);

        self.mask_timer_interrupts(false);
    }

    #[inline]
    fn mask_timer_interrupts(&self, mask: bool) {
        let val = self.read_reg(Self::TIMER_LVT);
        if mask {
            self.write_reg(Self::TIMER_LVT, val | (1 << Self::TIMER_INTERRUPT));
        } else {
            self.write_reg(Self::TIMER_LVT, val & !(1 << Self::TIMER_INTERRUPT));
        }
    }

    #[inline]
    fn set_timer_mode(&self, mode: ApicTimerMode) {
        let mode = mode as u32;
        let old_val = self.read_reg(Self::TIMER_LVT);
        let new_val = (old_val & !(0b11 << 17)) | mode << 17;
        self.write_reg(Self::TIMER_LVT, new_val);
    }

    #[inline]
    fn set_timer_interrupt_vector(&self, vector: u8) {
        let old_val = self.read_reg(Self::TIMER_LVT);
        let new_val = (old_val & !0xff) | vector as u32;
        self.write_reg(Self::TIMER_LVT, new_val);
    }
    // #[inline]
    // fn unmask_timer_interrupts(&self) {
    //     let val = self.read_reg(Self::TIMER_LVT);
    //     self.write_reg(Self::TIMER_LVT, val & !(1 << Self::TIMER_INTERRUPT));
    // }

    #[inline]
    fn set_timer_divider(&self, divider: u8) {
        // divider = 2 ** (i+1) for i = 0..6
        // divider = 1 if i = 7
        let divider = match divider {
            1 => 0b111,
            2 => 0b000,
            4 => 0b001,
            8 => 0b010,
            16 => 0b011,
            32 => 0b100,
            64 => 0b101,
            128 => 0b110,
            _ => panic!("unsupported APIC Timer divider: {divider}"),
        };

        self.write_reg(Self::DIVIDER, divider);
    }

    #[inline]
    fn set_timer_initial_count(&self, init_count: u32) {
        self.write_reg(Self::INIT_COUNT, init_count);
    }

    #[inline]
    fn get_timer_current_count(&self) -> u32 {
        self.read_reg(Self::CURRENT_COUNT)
    }

    pub(in super::super) fn set_timer_initial_count_in_ns(&self, ns: u32) {
        // cast to u64 to avoid overflow while calculating
        let timer_freq_considering_divider = APIC_TIMER_FREQ.load(Ordering::Relaxed)
            / APIC_TIMER_DIVIDER.load(Ordering::Relaxed) as u64;
        let init_count = (timer_freq_considering_divider * ns as u64) / 10u64.pow(9);
        assert!(init_count < u32::MAX as u64);
        self.set_timer_initial_count(init_count as u32);
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

#[repr(u32)]
enum ApicTimerMode {
    Oneshot = 0,
    Periodic = 1,
    // TscDeadline = 2, // not supported
}
