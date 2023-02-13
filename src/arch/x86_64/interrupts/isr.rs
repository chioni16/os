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

pub(super) extern "C" fn divide_by_zero(isf: &InterruptStackFrame) -> ! {
    crate::println!("{:?}", isf);
    crate::println!("EXCEPTION: DIVIDE BY ZERO @ {}", isf.ip);
    loop {}
}

pub(super) extern "C" fn breakpoint(isf: &InterruptStackFrame) {
    crate::println!("{:?}", isf);
    crate::println!("EXCEPTION: BREAKPOINT @ {:x}", isf.ip);
}

pub(super) extern "C" fn invalid_opcode(isf: &InterruptStackFrame) {
    crate::println!("{:?}", isf);
    crate::println!("EXCEPTION: INVALID OPCODE @ {:}", isf.ip);
}

pub(super) extern "C" fn page_fault(isf: &InterruptStackFrame, error_code: u64) -> ! {
    crate::println!("{:?}", isf);
    crate::println!("EXCEPTION: PAGE FAULT @ {}", isf.ip);
    crate::println!("{}", error_code);
    loop {}
}
