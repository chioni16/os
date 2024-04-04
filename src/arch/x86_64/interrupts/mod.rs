mod idt;
mod isr;

use idt::InterruptDescriptorTable;
use lazy_static::lazy_static;
use paste::paste;

use isr::*;

macro_rules! handler {
    ($int: ident) => {{
        paste! {
            #[naked]
            extern "C" fn [<stub_$int>]() -> ! {
                unsafe {
                    core::arch::asm!(
                        "push rax",
                        "push rcx",
                        "push rdx",
                        "push rsi",
                        "push rdi",
                        "push r8",
                        "push r9",
                        "push r10",
                        "push r11",

                        "mov rdi, rsp",
                        "add rdi, 9*8",
                        "call {}",

                        "pop r11",
                        "pop r10",
                        "pop r9",
                        "pop r8",
                        "pop rdi",
                        "pop rsi",
                        "pop rdx",
                        "pop rcx",
                        "pop rax",

                        "iretq",
                        sym $int,
                        options(noreturn),
                    );
                }
            }
            use crate::arch::x86_64::interrupts::isr::HandlerFn;
            [<stub_$int>] as HandlerFn
        }
    }};
}

// #TODO merge the two macros into one
// if not possible using declarative macros, use proc macros
macro_rules! handler_with_error_code {
    ($int: ident) => {{
        paste! {
            #[naked]
            extern "C" fn [<stub_$int>]() -> ! {
                unsafe {
                    core::arch::asm!(
                        "pop rsi",

                        "push rax",
                        "push rcx",
                        "push rdx",
                        "push rsi",
                        "push rdi",
                        "push r8",
                        "push r9",
                        "push r10",
                        "push r11",

                        "mov rdi, rsp",
                        "add rdi, 9*8",
                        "call {}",

                        "pop r11",
                        "pop r10",
                        "pop r9",
                        "pop r8",
                        "pop rdi",
                        "pop rsi",
                        "pop rdx",
                        "pop rcx",
                        "pop rax",

                        "iretq",
                        sym $int,
                        options(noreturn),
                    );
                }
            }
            use crate::arch::x86_64::interrupts::isr::HandlerFn;
            [<stub_$int>] as HandlerFn
        }
    }};
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.add_handler(0x0, handler!(divide_by_zero));
        idt.add_handler(0x3, handler!(breakpoint));
        idt.add_handler(0x6, handler!(invalid_opcode));
        idt.add_handler(0xe, handler_with_error_code!(page_fault));

        idt.add_handler(0x20, handler!(timer));
        idt.add_handler(0x21, handler!(keyboard));

        idt.add_handler(0x2b, handler!(rx_handler));
        idt
    };
}

pub(super) fn init() {
    IDT.load();
}
