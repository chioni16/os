mod character;
mod writer;

use character::{ ColourCode, Colour } ;
use writer::Writer;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    static ref WRITER: Mutex<Writer> 
        = Mutex::new(Writer::new(ColourCode::new(Colour::Yellow, Colour::Black)));
}

pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}


