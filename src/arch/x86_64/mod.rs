mod acpi;
mod interrupts;
mod pic;
mod port;
mod vga_buffer;

pub(crate) fn init() {
    interrupts::init();
    pic::init();
    // let _rsdt = acpi::find_rsdt().unwrap();
}

pub(crate) use vga_buffer::_print;
