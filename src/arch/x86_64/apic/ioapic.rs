use super::acpi::{self, IoApicIntSourceOverride};
use crate::mem::PhysicalAddress;
use log::{info, trace};

pub(super) struct IoApic(acpi::IoApic);

impl IoApic {
    pub(super) fn new(table: acpi::IoApic) -> Self {
        Self(table)
    }

    pub(super) fn init(&self, overrides: impl Iterator<Item = IoApicIntSourceOverride>) {
        info!("Initialising IOAPIC {}", self.0.ioaid);

        // TODO: Use AML to discern mappings not available in overrides
        // setting some default entries (gsi = irq), eg: keyboard
        let default_entries = [IoApicIntSourceOverride {
            bus_source: 0x0,
            irq_source: 0x1,
            gsi: 0x1,
            flags: 0x0,
        }];

        let ioapicver = self.read_reg(1);
        let num_gsi = (ioapicver >> 16) as u8 + 1;
        // overrides are chained to default_entries and not the other way around
        // as I want the overrides to be fiven priority in case of common entries
        default_entries
            .into_iter()
            .chain(overrides)
            .filter(|&or| self.0.gsib <= or.gsi && or.gsi < self.0.gsib + num_gsi as u32)
            .for_each(|or| {
                trace!("got override: {:#x?}", or);
                let index = or.gsi - self.0.gsib;
                let int_vec = or.irq_source + 0x20;
                let pin_polarity = or.flags & 0x2 != 0;
                let trigger_mode = or.flags & 0x8 != 0;
                // TODO: you may want to disable certain interrupts
                let disable = false;
                // TODO: need to change this when SMP is in play
                let destination = 0;

                let mut val = ((destination as u64) << 56) | int_vec as u64;
                if pin_polarity {
                    val |= 1 << 13;
                }
                if trigger_mode {
                    val |= 1 << 15;
                }
                if disable {
                    val |= 1 << 16;
                }
                self.write_ioredtbl(index, val);
            });

        trace!("    version: {}", ioapicver & 0xff);
        trace!("    num of irqs: {}", num_gsi);
        for index in 0..num_gsi as u32 {
            let entry = self.read_ioredtbl(index);
            trace!("    IRQ {}", index);
            trace!("        vector: {}", entry as u8);
            trace!("        delivery mode: {:#b}", (entry >> 8) & 0b111);
            trace!("        destination mode: {}", (entry >> 11) & 1);
            trace!("        delivery status: {}", (entry >> 12) & 1);
            trace!("        pin polarity: {}", (entry >> 13) & 1);
            trace!("        remote IRR: {}", (entry >> 14) & 1);
            trace!("        trigger mode: {}", (entry >> 15) & 1);
            trace!("        mask: {}", (entry >> 16) & 1);
            trace!("        destination: {:#b}", (entry >> 56) as u8);
        }
    }

    #[inline]
    fn write_ioregsel(&self, index: u32) {
        unsafe {
            let addr = PhysicalAddress::new(self.0.ioapic_addr as u64)
                .to_virt()
                .unwrap()
                .to_inner() as *mut u32;
            core::intrinsics::volatile_store(addr, index);
        }
    }

    #[inline]
    fn write_ioregwin(&self, val: u32) {
        unsafe {
            let addr = PhysicalAddress::new(self.0.ioapic_addr as u64 + 0x10)
                .to_virt()
                .unwrap()
                .to_inner() as *mut u32;
            core::intrinsics::volatile_store(addr, val);
        }
    }

    #[inline]
    fn read_ioregwin(&self) -> u32 {
        unsafe {
            let addr = PhysicalAddress::new(self.0.ioapic_addr as u64 + 0x10)
                .to_virt()
                .unwrap()
                .to_inner() as *mut u32;
            core::intrinsics::volatile_load(addr)
        }
    }

    #[inline]
    fn read_reg(&self, index: u32) -> u32 {
        self.write_ioregsel(index);
        self.read_ioregwin()
    }

    #[inline]
    fn write_reg(&self, index: u32, val: u32) {
        self.write_ioregsel(index);
        self.write_ioregwin(val);
    }

    #[inline]
    fn read_ioredtbl(&self, index: u32) -> u64 {
        let lower_index = 0x10 + 2 * index;
        self.write_ioregsel(lower_index);
        let lower_val = self.read_ioregwin();

        let higher_index = 0x10 + 2 * index + 1;
        self.write_ioregsel(higher_index);
        let higher_val = self.read_ioregwin();

        ((higher_val as u64) << 32) | lower_val as u64
    }

    #[inline]
    fn write_ioredtbl(&self, index: u32, val: u64) {
        let lower_val = (val & 0xffffffff) as u32;
        let higher_val = (val > 32) as u32;

        let lower_index = 0x10 + 2 * index;
        self.write_ioregsel(lower_index);
        self.write_ioregwin(lower_val);

        let higher_index = 0x10 + 2 * index + 1;
        self.write_ioregsel(higher_index);
        self.write_ioregwin(higher_val);
    }
}
