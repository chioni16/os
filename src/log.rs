#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        const VGA: bool = true;
        const SERIAL: bool = true;

        if VGA { crate::arch::_print(format_args!($($arg)*)); }
        if SERIAL { crate::log::_print_port(format_args!($($arg)*)); }
    });
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

struct PortWriter(u16);
static mut PW: PortWriter = PortWriter(0x3f8);

impl PortWriter {
    fn write_byte(&self, b: u8) {
        unsafe {
            core::arch::asm!("out dx, al", in("dx") self.0, in("al") b);
        }
    }
}

impl core::fmt::Write for PortWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
        Ok(())
    }
}

pub fn _print_port(args: core::fmt::Arguments) {
    use core::fmt::Write;
    unsafe {
        PW.write_fmt(args).unwrap();
    }
}
