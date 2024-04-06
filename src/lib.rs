#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(const_option)]

mod arch;
mod locks;
mod mem;
mod multiboot;

use core::{ops::Deref, ptr::addr_of};
use locks::SpinLock;
use mem::allocator::bitmap_allocator::BitMapAllocator;

#[global_allocator]
static HEAP_ALLOCATOR: SpinLock<BitMapAllocator> = BitMapAllocator::locked();

extern "C" {
    static stack_bottom: u8;
    static stack_top: u8;
}

#[no_mangle]
pub extern "C" fn rust_start(multiboot_addr: u64) -> ! {
    unsafe {
        println!("top: {:#x?}", addr_of!(stack_top));
        println!("bottom: {:#x?}", addr_of!(stack_bottom));
    }
    println!("Hello!: {:#x}", multiboot_addr);

    let multiboot_info = multiboot::MultibootInfo::new(multiboot_addr);

    // let a: u64 = 0;
    // unsafe {
    //     core::arch::asm!("div {}", in(reg) a);
    // }

    // unsafe { core::arch::asm!("ud2") };

    HEAP_ALLOCATOR.lock().init(&multiboot_info);
    arch::init();

    crate::println!("init done");

    unsafe {
        core::arch::asm!("int 3");
    }

    // unsafe {
    //     // let _ = core::ptr::read_volatile(0x40000000 as *const u8);
    //     core::ptr::write_volatile(0x40000000 as *mut u8, 0);
    // }

    println!("Bye!");

    loop {
        unsafe {
            core::arch::asm!("sti");
            core::arch::asm!("hlt");
        }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        const VGA: bool = true;
        const SERIAL: bool = true;

        if VGA { crate::arch::_print(format_args!($($arg)*)); }
        if SERIAL { crate::_print_port(format_args!($($arg)*)); }
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
