#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(const_option)]


mod arch;
mod mem;
mod multiboot;

use core::ptr::addr_of;

use buddy_system_allocator::LockedHeap;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

// use linked_list_allocator::LockedHeap;

// #[global_allocator]
// static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

extern "C" {
    static stack_bottom: u8;
    static stack_top: u8;
}

#[no_mangle]
pub extern "C" fn rust_start(multiboot_addr: u64) -> ! {
    println!("Hello!: {:#x}", multiboot_addr);
    unsafe {
        println!("top: {:#x?}", addr_of!(stack_top));
        println!("bottom: {:#x?}", addr_of!(stack_bottom));
    }

    let multiboot_info = multiboot::MultibootInfo::new(multiboot_addr);

    let elf_sections_tag = multiboot_info
        .multiboot_elf_tags()
        .unwrap()
        .filter(|s| s.section_type() != 0);
    let kernel_start = elf_sections_tag
        .clone()
        .map(|s| s.base_addr())
        .min()
        .unwrap();
    let kernel_end = elf_sections_tag
        .map(|s| s.base_addr() + s.size())
        .max()
        .unwrap();
    crate::println!(
        "kernel start: {:x}, kernel end: {:x}",
        kernel_start,
        kernel_end
    );

    crate::println!(
        "multiboot start: {:x}, multiboot end: {:x}",
        multiboot_info.start(),
        multiboot_info.end()
    );

    let heap_start = 0x800000;
    // let heap_start = 0x29800000 as *mut u8;
    let heap_size = 64 * 1024 * 1024;

    unsafe {
        HEAP_ALLOCATOR.lock().init(heap_start, heap_size);
    }
    println!("Hello again!");
    arch::init();
    println!("some numbers: {}", 42);

    // let a: u64 = 0;
    // unsafe {
    //     core::arch::asm!("div {}", in(reg) a);
    // }

    // unsafe { core::arch::asm!("ud2") };

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
