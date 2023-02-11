#![no_std]
#![no_main]

#![feature(naked_functions)]

mod arch;

#[no_mangle]
pub extern "C" fn rust_start() -> ! {
    arch::init();
    println!("Hello again!");
    println!("some numbers: {}", 42);

    // let a: u64 = 0;
    // unsafe {
    //     core::arch::asm!("div {}", in(reg) a);
    // }

    // unsafe { core::arch::asm!("ud2") };
    
    unsafe {
        core::arch::asm!("int 3");
    }

    unsafe {
        // let _ = core::ptr::read_volatile(0x40000000 as *const u8);
        core::ptr::write_volatile(0x40000000 as *mut u8, 0);
    }


    println!("Bye!");

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