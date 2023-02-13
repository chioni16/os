const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

use super::character::{ColourCode, ScreenChar};
use core::fmt;

struct Buffer;

impl Buffer {
    const VGA_BUFFER: *mut ScreenChar = 0xb8000 as *mut ScreenChar;

    unsafe fn get_addr(row: usize, col: usize) -> *mut ScreenChar {
        unsafe { Self::VGA_BUFFER.add(row * BUFFER_WIDTH + col) }
    }

    fn read(&self, row: usize, col: usize) -> ScreenChar {
        unsafe {
            let addr = Self::get_addr(row, col);
            core::ptr::read_volatile(addr)
        }
    }

    fn write(&self, row: usize, col: usize, val: ScreenChar) {
        unsafe {
            let addr = Self::get_addr(row, col);
            core::ptr::write_volatile(addr, val);
        }
    }
}

pub(super) struct Writer {
    column_position: usize,
    colour_code: ColourCode,
    buffer: Buffer,
}

impl Writer {
    pub(super) fn new(colour_code: ColourCode) -> Self {
        Self {
            column_position: 0,
            colour_code,
            buffer: Buffer,
        }
    }

    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.colour_code;
                self.buffer.write(
                    row,
                    col,
                    ScreenChar {
                        ascii_character: byte,
                        color_code,
                    },
                );
                self.column_position += 1;
            }
        }
    }

    fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let sc = self.buffer.read(row, col);
                self.buffer.write(row - 1, col, sc);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.colour_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.write(row, col, blank);
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}
