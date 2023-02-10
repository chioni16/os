#![no_std]
#![no_main]

mod interrupts;
mod vga_buffer;

#[no_mangle]
pub extern "C" fn rust_start() -> ! {
    interrupts::init();
    println!("Hello again!");
    println!("some numbers: {}", 42);
    loop {}
} 

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
