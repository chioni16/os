#![no_std]
#![no_main]

mod arch;

#[no_mangle]
pub extern "C" fn rust_start() -> ! {
    arch::init();
    println!("Hello again!");
    println!("some numbers: {}", 42);
    loop {}
} 

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => (crate::arch::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
