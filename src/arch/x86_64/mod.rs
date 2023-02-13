mod apic;
mod interrupts;
mod vga_buffer;

pub(crate) fn init() {
    interrupts::init();
    let _rsdt = apic::find_rsdt();
}

pub(crate) use vga_buffer::_print;
