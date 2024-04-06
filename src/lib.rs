#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(const_option)]

mod arch;
mod locks;
mod log;
mod mem;
mod multiboot;

use core::ptr::addr_of;
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
