mod character;
pub mod writer;

pub use writer::WRITER;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => (crate::vga_buffer::writer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}