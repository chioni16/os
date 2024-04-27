#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(const_option)]
#![feature(core_intrinsics)]
#![feature(let_chains)]

mod arch;
mod locks;
mod logging;
mod mem;
mod multiboot;

#[macro_use]
extern crate alloc;
use core::ptr::addr_of;
use locks::SpinLock;
use mem::allocator::bitmap_allocator::BitMapAllocator;

#[global_allocator]
static HEAP_ALLOCATOR: SpinLock<BitMapAllocator> = BitMapAllocator::locked();

use log::{info, trace, LevelFilter};
use logging::Logger;

use crate::arch::ACTIVE_PAGETABLE;
static LOGGER: Logger = Logger;

// Weird behaviour:
// before call rust_start:
// ffff800004212000: 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00
// after call rust_start:
// ffff800004212000: 0x00 0x80 0xff 0xff 0x00 0x00 0x00 0x00
static HEAP_ALLOCATOR2: SpinLock<BitMapAllocator> = BitMapAllocator::locked();

extern "C" {
    static stack_bottom: u8;
    static stack_top: u8;
    static HIGHER_HALF: u8;
    static mut gdt64: u8;
}

// static mut HIGHER_HALF_ADDRESS: u64 = 0x0;

#[no_mangle]
pub extern "C" fn rust_start(multiboot_addr: u64) -> ! {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Info))
        .unwrap();

    unsafe {
        println!("top: {:#x?}", addr_of!(stack_top));
        println!("bottom: {:#x?}", addr_of!(stack_bottom));
        println!("higher half: {:#x?}", addr_of!(HIGHER_HALF));
    }

    // unsafe {
    //     HIGHER_HALF_ADDRESS = core::ptr::addr_of!(crate::HIGHER_HALF) as u64;
    //     crate::println!("ptr: {:#x}", HIGHER_HALF_ADDRESS);
    // }

    ACTIVE_PAGETABLE.lock().init();
    trace!("multiboot_addr: {:#x}", multiboot_addr);
    let multiboot_info = multiboot::MultibootInfo::new(multiboot_addr);
    // HEAP_ALLOCATOR2.lock().init(&multiboot_info);
    HEAP_ALLOCATOR.lock().init(&multiboot_info);
    arch::init(&multiboot_info);
    info!("init done");

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
