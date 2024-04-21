mod idt;
mod isr;

use idt::InterruptDescriptorTable;
use lazy_static::lazy_static;
use paste::paste;

use isr::*;
pub use isr::{disable_interrupts, enable_interrupts, is_int_enabled};

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

pub(super) const SYSCALL_HANDLER: usize = 0x2e;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.add_handler(0x0, handler!(divide_by_zero), 0, 0);
        idt.add_handler(0x3, handler!(breakpoint), 0, 0);
        idt.add_handler(0x6, handler!(invalid_opcode), 0, 0);
        idt.add_handler(0xe, handler_with_error_code!(page_fault), 0, 0);

        idt.add_handler(0x20, handler!(timer), 0, 0);
        idt.add_handler(0x21, handler!(keyboard), 0, 0);
        idt.add_handler(0x2b, handler!(rx_handler), 0, 0);

        idt.add_handler(SYSCALL_HANDLER, handler!(syscall), 0, 3);

        idt.add_handler(0x2f, handler!(hpet), 0, 0);
        idt
    };
}

pub(super) fn init() {
    IDT.load();
}
