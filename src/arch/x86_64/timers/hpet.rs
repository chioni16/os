use alloc::vec::Vec;
use log::{info, trace};

use crate::{
    arch::x86_64::{acpi::HpetEntry, paging::mmio},
    mem::{PhysicalAddress, VirtualAddress},
};

#[derive(Debug)]
pub(in super::super) struct Hpet {
    table: HpetEntry,
    base: PhysicalAddress,
    period: u64,
    support_periodic: Vec<u8>,
}

impl Hpet {
    pub(super) fn new(table: HpetEntry) -> Self {
        let hpet = Self {
            base: PhysicalAddress::new(table.address),
            table,
            // initialised later when `init` is called
            period: 0,
            support_periodic: Vec::new(),
        };

        info!("Created new HPET: {:#x?}", hpet);
        hpet
    }

    // SAFETY: this is to be called only once
    // and never when the HPET has already started counting
    pub(super) unsafe fn init(&mut self) {
        // map the MMIO regs used by HPET
        // https://www.intel.com/content/dam/www/public/us/en/documents/technical-specifications/software-developers-hpet-spec-1-0a.pdf
        mmio::map(self.base, self.base.offset(0x3ff));

        let gen_cap_id = self.read_reg(0x0);
        let period = gen_cap_id >> 32;
        self.period = period;
        trace!("HPET period: {}", period);
        trace!("HPET frequency: {}", self.frequency());

        // initialise counter
        self.write_main_counter(23);
        info!("HPET main counter set to {}", self.read_main_counter());

        // initialise each comparator individually
        for n in 0..self.table.comparator_count() {
            // do some basic initialisation
            let conf_offset = self.timer_conf_reg_offset(n as usize);
            let conf_val = self.read_reg(conf_offset);

            // get routing capabilities
            let gsids = (conf_val >> 32) as u32;
            // TODO: set routing
            // currently takes shortcut and works only if the IOAPIC gsid 2 is available for mapping
            assert!(gsids & 0b100 != 0);
            trace!("comparator {} allows IOAPIC entries {:#b}", n, gsids);

            // get periodic mode capable timers
            let allows_periodic = conf_val & (1 << 4) != 0;
            trace!("comparator {} allows periodic mode {}", n, allows_periodic);
            // TODO: pass this information to IOAPIC initialisation code to properly set the GSID -> IRQ mapping
            // currently takes shortcut and works only if all the timers support periodic mode
            // especially timer 0, which is used to calibrate LAPIC timers
            assert!(allows_periodic);
            if allows_periodic {
                self.support_periodic.push(n);
            }

            // disable interupts for the timer
            self.write_reg(conf_offset, 0);
        }

        // enable interrupts in legacy routing mode
        // turns off PIT and RTC
        // The main intention of using HPET is to calibrate LAPIC timers
        // So, the legacy routing mode should work for our purposes
        let config = self.read_reg(0x10);
        self.write_reg(0x10, config | 0b11);
    }

    #[inline]
    fn frequency(&self) -> u64 {
        10u64.pow(15) / self.period
    }

    // convert time in ns to the equivalent value in counter
    #[inline]
    pub(in super::super) fn ns_to_counter(&self, ns: u64) -> u64 {
        (ns * self.frequency()) / 10u64.pow(9)
    }

    #[inline]
    fn read_reg(&self, offset: u64) -> u64 {
        unsafe {
            let base = self.base.to_virt().unwrap();
            let addr = base.offset(offset).as_mut_ptr();
            core::intrinsics::volatile_load(addr)
        }
    }

    #[inline]
    fn write_reg(&self, offset: u64, val: u64) {
        unsafe {
            let base = self.base.to_virt().unwrap();
            let addr = base.offset(offset).as_mut_ptr();
            core::intrinsics::volatile_store(addr, val);
        }
    }

    #[inline]
    pub(in super::super) fn read_main_counter(&self) -> u64 {
        self.read_reg(0xf0)
    }

    #[inline]
    pub(in super::super) fn write_main_counter(&self, val: u64) {
        self.write_reg(0xf0, val);
    }

    #[inline]
    fn timer_conf_reg_offset(&self, n: usize) -> u64 {
        0x100 + 0x20 * n as u64
    }

    #[inline]
    fn timer_comparator_val_reg_offset(&self, n: usize) -> u64 {
        0x108 + 0x20 * n as u64
    }

    pub(in super::super) fn enable_timer_oneshot(&self, n: usize, ns: u64, ioapic_interrupt: u8) {
        let counter = self.ns_to_counter(ns);
        assert!(counter > 0);

        let conf_offset = self.timer_conf_reg_offset(n);
        // set ioapic gsid to be used for interrupts and enable interrupts
        let val = (ioapic_interrupt as u64) << 9 | 1 << 0x2;
        self.write_reg(conf_offset, val);
        // if the value written is legal, we should be able to read the same value back
        assert_eq!(
            self.read_reg(conf_offset) & (0b11111 << 9),
            (ioapic_interrupt as u64) << 9
        );

        let comp_offset = self.timer_comparator_val_reg_offset(n);
        self.write_reg(comp_offset, self.read_main_counter() + counter);
    }

    pub(in super::super) fn enable_timer_periodic(&self, n: usize, ns: u64, ioapic_interrupt: u8) {
        let counter = self.ns_to_counter(ns);
        assert!(counter > 0);

        let conf_offset = self.timer_conf_reg_offset(n);
        // set ioapic gsid to be used for interrupts and enable interrupts in periodic mode
        // setting 6th bit allows us to modify the inner counter directly
        // workaround to deal with the fact that the period is set to the last value written to the main counter
        // bit 6 automatically clears after the first write?
        // PS: I don't fully understand it yet
        let val = (ioapic_interrupt as u64) << 9 | 1 << 6 | 1 << 3 | 1 << 0x2;
        self.write_reg(conf_offset, val);
        // if the value written is legal, we should be able to read the same value back
        assert_eq!(
            self.read_reg(conf_offset) & (0b11111 << 9),
            (ioapic_interrupt as u64) << 9
        );

        let comp_offset = self.timer_comparator_val_reg_offset(n);
        self.write_reg(comp_offset, self.read_main_counter() + counter);
        self.write_reg(comp_offset, counter);
    }
}
