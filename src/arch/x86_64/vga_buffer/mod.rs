mod character;
mod writer;

use character::{Colour, ColourCode};
use lazy_static::lazy_static;
use spin::Mutex;
use writer::Writer;

lazy_static! {
    static ref WRITER: Mutex<Writer> =
        Mutex::new(Writer::new(ColourCode::new(Colour::Yellow, Colour::Black)));
}

pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}
