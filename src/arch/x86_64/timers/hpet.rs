use crate::{arch::x86_64::acpi::HpetEntry, mem::{PhysicalAddress, VirtualAddress}};

struct Hpet {
    table: HpetEntry,
    base: VirtualAddress,
    period: u64,
}

impl Hpet {
    pub(super) fn new(table: HpetEntry) -> Self {
        Self { 
            base: unsafe { PhysicalAddress::new(table.address).to_virt().unwrap()},
            table,
            period: 0, // initialised later when `init` is called
        }
    }

    // SAFETY: this is to be called only once 
    // and never when the HPET has already started counting
    pub(super) unsafe fn init(&mut self) {
        let gen_cap_id = self.read_reg(0x0);
        let period = gen_cap_id >> 32;
        self.period = period;
        let frequency = 10u64.pow(15) / period;
        // initialise counter

        // initialise each comparator individually
        for comparator in 0..self.table.comparator_count() {
            // do some basic initialisation

            // set routing
        }

        // start main counter in legacy mapped mode
        let config = self.read_reg(0x10);
        self.write_reg(0x10, config | 0b11);
    }

    #[inline]
    fn read_reg(&self, offset: u64) -> u64 {
        unsafe { 
            let addr = self.base.offset(offset).as_mut_ptr();
            core::intrinsics::volatile_load(addr)
        }
    }

    #[inline]
    fn write_reg(&self, offset: u64, val: u64) {
        unsafe { 
            let addr = self.base.offset(offset).as_mut_ptr();
            core::intrinsics::volatile_store(addr, val);
        }
    }

    #[inline]
    fn read_main_counter(&self) -> u64 {
        self.read_reg(0xf0)
    }

    #[inline]
    fn write_main_counter(&self, val: u64) {
        self.write_reg(0xf0, val);
    }

    #[inline]
    fn timer_conf_reg_offset(&self, n: usize) -> VirtualAddress {
        let offset = 0x100 + 0x20 * n as u64;
        self.base.offset(offset)
    }

    #[inline]
    fn timer_comparator_val_reg_offset(&self, n: usize) -> VirtualAddress {
        let offset = 0x108 + 0x20 * n as u64;
        self.base.offset(offset)
    }

    fn enable_timer_oneshot(&self, n: usize, time: u64) {
        assert!(time >= self.period);

        let conf_offset = self.timer_comparator_val_reg_offset(n);
        self.write_reg(conf_offset.as_mut_ptr::<u8>() as _, todo!());
        let comp_offset = self.timer_comparator_val_reg_offset(n);
        self.write_reg(comp_offset.as_mut_ptr::<u8>() as _, self.read_main_counter() + time);
    }
}
