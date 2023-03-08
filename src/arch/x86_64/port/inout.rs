use super::asm::*;

pub(in super::super) trait InOut {
    unsafe fn port_in(port: u16) -> Self;
    unsafe fn port_out(port: u16, data: Self);
}

impl InOut for u8 {
    unsafe fn port_in(port: u16) -> Self {
        inb(port)
    }
    unsafe fn port_out(port: u16, data: Self) {
        outb(port, data)
    }
}

impl InOut for u16 {
    unsafe fn port_in(port: u16) -> Self {
        inw(port)
    }
    unsafe fn port_out(port: u16, data: Self) {
        outw(port, data)
    }
}

impl InOut for u32 {
    unsafe fn port_in(port: u16) -> Self {
        inl(port)
    }
    unsafe fn port_out(port: u16, data: Self) {
        outl(port, data)
    }
}
