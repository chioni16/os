mod create;
mod delay;
mod lock;
mod pid;
mod process;
mod scheduler;

use self::{
    create::{create_kernel_task, create_user_task},
    lock::Lock,
    pid::{get_new_pid, Pid},
    process::{Process, State},
    scheduler::Scheduler,
};
use super::{apic, timers::hpet::Hpet};
use crate::{
    arch::get_cur_page_table_start,
    mem::{PhysicalAddress, VirtualAddress},
    multiboot::MultibootInfo,
};
use alloc::sync::Arc;
use core::mem::MaybeUninit;
use log::info;

static SCHEDULER_LOCK: Lock = Lock::new();
static mut SCHEDULER: MaybeUninit<Scheduler> = MaybeUninit::uninit();

// to be used from within the task init functions
#[no_mangle]
unsafe fn scheduler_unlock() {
    SCHEDULER_LOCK.unlock();
}

#[no_mangle]
fn schedule() {
    let scheduler = unsafe { SCHEDULER.assume_init_mut() };

    SCHEDULER_LOCK.lock();
    // SAFETY: locking disables interrupts
    unsafe {
        scheduler.schedule();
    }
    // SAFETY: SCHEDULER_LOCK is locked just above
    unsafe {
        SCHEDULER_LOCK.unlock();
    }
}

#[no_mangle]
fn delay(delay_ns: u64) {
    let scheduler = unsafe { SCHEDULER.assume_init_mut() };

    SCHEDULER_LOCK.lock();

    scheduler.block_current();
    scheduler.delays.add(scheduler.cur_proc, delay_ns);

    // SAFETY: locking disables interrupts
    unsafe {
        scheduler.schedule();
    }
    // SAFETY: SCHEDULER_LOCK is locked just above
    unsafe {
        SCHEDULER_LOCK.unlock();
    }
}

#[no_mangle]
fn unblock(pid: u64) {
    let pid = Pid(pid as u32);

    let scheduler = unsafe { SCHEDULER.assume_init_mut() };

    SCHEDULER_LOCK.lock();

    // info!("scheduler: {:#x?}", scheduler);
    scheduler.unblock(pid);

    // SAFETY: locking disables interrupts
    unsafe {
        scheduler.schedule();
    }
    // SAFETY: SCHEDULER_LOCK is locked just above
    unsafe {
        SCHEDULER_LOCK.unlock();
    }
}

pub(super) fn timer_interrupt_handler() {
    let scheduler = unsafe { SCHEDULER.assume_init_mut() };

    SCHEDULER_LOCK.lock();

    // SAFETY: ensures that the EOI is sent to LAPIC
    // the goal is to send EOI once the interrupts are disabled
    // why is that necessary? I am not sure. I just felt that it's better if I do it once the interrupts are disabled
    // if something goes wrong, you know whom to blame ;)
    unsafe {
        apic::send_eoi();
    }

    for pid in scheduler.delays.get_expired_timers() {
        scheduler.unblock(pid);
    }

    // SAFETY: locking disables interrupts
    unsafe {
        scheduler.schedule();
    }

    // SAFETY: SCHEDULER_LOCK is locked just above
    unsafe {
        SCHEDULER_LOCK.unlock();
    }
}

pub(super) fn init(multiboot_info: &MultibootInfo, hpet: Arc<Hpet>) {
    let mut init = Process {
        id: get_new_pid(),
        // SAFETY: paging enabled by the time we get here
        cr3: unsafe { get_cur_page_table_start() },
        stack_top: VirtualAddress::new(0),
        kernel_stack_top: VirtualAddress::new(0), // doesn't use this
        state: State::Running,
    };
    init.stack_top = unsafe {
        let stack_top: u64;
        core::arch::asm!(
            "mov {tmp}, rsp",
            tmp = out(reg) stack_top,
            options(nostack)
        );
        VirtualAddress::new(stack_top)
    };

    let mut scheduler = Scheduler::new(init, hpet);

    let p0 = create_kernel_task(func0 as _);
    info!("p0: {:#x?}", p0);
    scheduler.add(p0);
    // let p1 = create_user_task(func1 as _);
    // info!("p1: {:#x?}", p1);
    // scheduler.add(p1);
    // let p2 = create_user_task(func2 as _);
    // info!("p2: {:#x?}", p2);
    // scheduler.add(p2);

    for module in multiboot_info.multiboot_modules() {
        info!("[scheduler init] module: {:#x?}", module);
        let (entry, page_table) = super::elf::load_elf(
            PhysicalAddress::new(module.mod_start as u64),
            (module.mod_end - module.mod_start) as usize
        );
        let proc = create::create_user_task2(entry, page_table);
        scheduler.add(proc);
    }

    info!("scheduler: {:#x?}", scheduler);

    unsafe {
        SCHEDULER.write(scheduler);
    }
    loop {
        schedule();
    }
}

// SAFETY: interrupts need to be disabled before calling this function
#[no_mangle]
#[naked]
unsafe extern "sysv64" fn task_switch() {
    core::arch::asm!(
        // Save current process's state
        // RIP is also already saved on the stack by the `call` instruction

        // WARNING: not saving the segment registers for all the kernel space tasks
        // also the TSS rsp0 remains the same value for the given task throughout its lifetime
        // The idea is whenever the kernel stack is used, it regains its original position after usage
        // TODO: need to reevaluate this assumption and store this value if assumption is proved wrong
        // similarly, no need to save the `cr3` value that contains the physical address of the level 4 page table for the task
        // It remains the same throughout task's lifetime.

        // no need to save the caller-saved registers
        // save the callee saved regs on the current stack
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        // current process's control block passed as the first parameter when this function is called
        // As per the calling convention used (SystemV64), this is passed through "rdi" register
        // similarly, the next process's control block is passed as the second argument. "rsi" is used for this.

        // save the rsp in the process control block
        "mov [rdi + 0x00], rsp",
        // start loading the next task's state
        "mov rax, [rsi + 0x10]",
        "mov rcx, cr3",
        "cmp rax, rcx",
        "je 2f", // avoid flushing TLB unnecessarily
        "mov cr3, rax",
        "2:",
        "mov rsp, [rsi + 0x00]",
        "mov rax, [rsi + 0x08]",
        // the address of TSS is passed as the third argument,
        // which is stored in `rdx` register as per the calling convention
        "mov dword ptr [rdx + 0x04], eax", // lower 32 bits
        "shr rax, 32",
        "mov dword ptr [rdx + 0x08], eax", // higher 32 bits
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        "ret",
        options(noreturn)
    )
}

#[naked]
extern "C" fn func0() {
    unsafe {
        core::arch::asm!(
            "3:",
            "mov r8, 0xffff8000000b8000",
            "mov byte ptr [r8], 0x30",
            "mov r8, 0",
            "4:",
            "add r8, 1",
            "cmp r8, 100000000",
            "jl 4b",
            "jmp 3b",
            options(noreturn),
        );
    }
}
#[naked]
extern "C" fn func1() {
    unsafe {
        core::arch::asm!(
            "3:",
            // "mov r8, 0xffff8000000b8000",
            // "mov byte ptr [r8], 0x31",
            "mov r11, 1",
            "int 0x2e",
            "push rax",
            "pop rax",
            "mov r8, 0",
            "4:",
            "add r8, 1",
            "cmp r8, 100000000",
            "jl 4b",
            // "call schedule",
            // "mov rdi, 2",
            // "call unblock",
            // "mov rdi, 1000000000",
            // "call delay",
            "jmp 3b",
            options(noreturn),
        );
    }
}

#[naked]
extern "C" fn func2() {
    unsafe {
        core::arch::asm!(
            "5:",
            // "mov r8, 0xffff8000000b8000",
            // "mov byte ptr [r8], 0x32",
            "mov r11, 2",
            "int 0x2e",
            "mov r8, 0",
            "6:",
            "push rax",
            "pop rax",
            "add r8, 1",
            "cmp r8, 100000000",
            "jl 6b",
            // "call schedule",
            // "call block",
            // "mov rdi, 2000000000",
            // "call delay",
            "jmp 5b",
            options(noreturn),
        );
    }
}
