use crate::arch::x86_64::{apic, port::Port};

pub(super) type HandlerFn = extern "C" fn() -> !;

#[derive(Debug)]
#[repr(C)]
pub(super) struct InterruptStackFrame {
    ip: u64,
    cs: u64, // padded
    rflags: u64,
    sp: u64,
    ss: u64, // padded
}

// #[naked]
// pub(super) extern "C" fn breakpoint_handler() -> ! {
//     unsafe {
//         core::arch::asm!(
//             "push rax",
//             "push rcx",
//             "push rdx",
//             "push rsi",
//             "push rdi",
//             "push r8",
//             "push r9",
//             "push r10",
//             "push r11",

//             "mov rdi, rsp",
//             "add rdi, 9*8",
//             "call {}",

//             "pop rax",
//             "pop rcx",
//             "pop rdx",
//             "pop rsi",
//             "pop rdi",
//             "pop r8",
//             "pop r9",
//             "pop r10",
//             "pop r11",

//             "iretq",
//             sym _breakpoint_handeler,
//             options(noreturn),
//         );
//     }
// }

#[inline]
pub fn is_int_enabled() -> bool {
    let flags: u64;
    unsafe {
        core::arch::asm!(
            "pushfq",
            "pop {}",
            out(reg) flags
        )
    };
    flags & (1 << 9) != 0
}

#[inline]
pub fn enable_interrupts() {
    unsafe {
        core::arch::asm!("sti");
    }
}

#[inline]
pub fn disable_interrupts() {
    unsafe {
        core::arch::asm!("cli");
    }
}

fn without_interrupts<R, F: FnOnce() -> R>(f: F) -> R {
    let int_enabled = is_int_enabled();
    if int_enabled {
        disable_interrupts();
    }
    let ret = f();
    if int_enabled {
        enable_interrupts();
    }
    ret
}

// get address that caused page fault
// this information is stored in the CR2 register
fn get_faulty_address() -> usize {
    let faulty_addr;
    unsafe {
        core::arch::asm!("mov {}, cr2", out(reg) faulty_addr, options(nostack));
    }
    faulty_addr
}

pub(super) extern "C" fn divide_by_zero(isf: &InterruptStackFrame) -> ! {
    crate::println!("{:?}", isf);
    crate::println!("EXCEPTION: DIVIDE BY ZERO @ {:#x}", isf.ip);
    loop {}
}

pub(super) extern "C" fn breakpoint(isf: &InterruptStackFrame) {
    crate::println!("{:?}", isf);
    crate::println!("EXCEPTION: BREAKPOINT @ {:#x}", isf.ip);
}

pub(super) extern "C" fn invalid_opcode(isf: &InterruptStackFrame) {
    crate::println!("{:?}", isf);
    crate::println!("EXCEPTION: INVALID OPCODE @ {:#x}", isf.ip);
}

pub(super) extern "C" fn page_fault(isf: &InterruptStackFrame, error_code: u64) -> ! {
    crate::println!("{:?}", isf);
    crate::println!(
        "EXCEPTION: PAGE FAULT accessing addr: {:#x}; instruction located @ {:#x}",
        get_faulty_address(),
        isf.ip
    );
    crate::println!("{:#b}", error_code);
    loop {}
}
pub(super) extern "C" fn timer(_isf: &InterruptStackFrame) {
    without_interrupts(|| unsafe {
        crate::print!(".");
        // pic::send_eoi(0);
        apic::send_eoi();

        crate::arch::x86_64::process::schedule();
    });
}
pub(super) extern "C" fn hpet(_isf: &InterruptStackFrame) {
    without_interrupts(|| unsafe {
        crate::println!("!");
        apic::send_eoi();
    });
}
pub(super) extern "C" fn keyboard(_isf: &InterruptStackFrame) {
    without_interrupts(|| unsafe {
        crate::print!("{:x}", Port::new(0x60).read::<u8>());
        // pic::send_eoi(1);
        apic::send_eoi();
    });
}

pub(super) fn rx_handler() {
    let base_addr = 0xc000u16;
    unsafe {
        // 0030h-0033h R/W RBSTART Receive (Rx) Buffer Start Address
        crate::println!("RBSTART: {:#x}", Port::new(base_addr + 0x30).read::<u32>());
        // 0037h R/W CR Command Register
        crate::println!("COMMAND: {:#b}", Port::new(base_addr + 0x37).read::<u8>());
        // 0038h-0039h R/W CAPR Current Address of Packet Read
        crate::println!("CAPR: {:#x}", Port::new(base_addr + 0x38).read::<u16>());
        // 003Ah-003Bh R CBR Current Buffer Address:
        crate::println!("CBA: {:#x}", Port::new(base_addr + 0x3a).read::<u16>());
        // 003Ch-003Dh R/W IMR Interrupt Mask Register
        crate::println!("IMR: {:#b}", Port::new(base_addr + 0x3c).read::<u16>());
        // 003Eh-003Fh R/W ISR Interrupt Status Register
        crate::println!("ISR: {:#b}", Port::new(base_addr + 0x3e).read::<u16>());
        // 0044h-0047h R/W RCR Receive (Rx) Configuration Register
        crate::println!("RCR: {:#x}", Port::new(base_addr + 0x44).read::<u32>());
    }
}

pub(super) fn syscall() {
    log::info!("SYSCALL handler");
}
