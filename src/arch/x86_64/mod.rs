mod interrupts;
mod vga_buffer;

pub(crate) fn init() {
    interrupts::init();
}

pub(crate) use vga_buffer::_print;