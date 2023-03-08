use core::arch::asm;

pub(super) unsafe fn inb(port: u16) -> u8 {
    let data: u8;
    asm!("in al, dx", in("dx") port, out("al") data, options(readonly, nostack));
    data
}
pub(super) unsafe fn inw(port: u16) -> u16 {
    let data: u16;
    asm!("in ax, dx", in("dx") port, out("ax") data, options(readonly, nostack));
    data
}
pub(super) unsafe fn inl(port: u16) -> u32 {
    let data: u32;
    asm!("in eax, dx", in("dx") port, out("eax") data, options(readonly, nostack));
    data
}

pub(super) unsafe fn outb(port: u16, data: u8) {
    asm!("out dx, al", in("dx") port, in("al") data);
}

pub(super) unsafe fn outw(port: u16, data: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") data);
}

pub(super) unsafe fn outl(port: u16, data: u32) {
    asm!("out dx, eax", in("dx") port, in("eax") data);
}
