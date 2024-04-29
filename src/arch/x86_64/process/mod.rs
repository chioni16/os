mod create;
mod delay;
mod lock;
mod pid;
mod process;
mod scheduler;

use self::{
    create::create_kernel_task,
    lock::Lock,
    pid::{get_new_pid, Pid},
    process::{Process, State},
    scheduler::Scheduler,
};
use crate::{arch::get_cur_page_table_start, mem::VirtualAddress};
use core::mem::MaybeUninit;
use log::info;

static SCHEDULER_LOCK: Lock = Lock::new();
static mut SCHEDULER: MaybeUninit<Scheduler> = MaybeUninit::uninit();

#[no_mangle]
pub(super) fn schedule() {
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
fn block() {
    let scheduler = unsafe { SCHEDULER.assume_init_mut() };

    SCHEDULER_LOCK.lock();

    // info!("scheduler: {:#x?}", scheduler);
    let task = scheduler.processes.get(&scheduler.cur_proc).unwrap();
    task.lock().state = State::Waiting;
    scheduler.ready_to_run -= 1;

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
    let task = scheduler.processes.get(&pid).unwrap();
    task.lock().state = State::Ready;
    scheduler.ready_to_run += 1;

    // SAFETY: locking disables interrupts
    unsafe {
        scheduler.schedule();
    }
    // SAFETY: SCHEDULER_LOCK is locked just above
    unsafe {
        SCHEDULER_LOCK.unlock();
    }
}

pub(super) fn init() {
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

    let p1 = create_kernel_task(func1 as _);
    info!("p1: {:#x?}", p1);
    let p2 = create_kernel_task(func2 as _);
    info!("p2: {:#x?}", p2);

    info!("Scheduler alloc started");
    let mut scheduler = Scheduler::new(init);
    scheduler.add(p1);
    scheduler.add(p2);
    info!("Scheduler alloc stopped");

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
extern "C" fn func1() {
    unsafe {
        core::arch::asm!(
            "3:",
            "mov r8, 0xffff8000000b8000",
            "mov byte ptr [r8], 0x31",
            "mov r8, 0",
            "4:",
            "add r8, 1",
            "cmp r8, 100000000",
            "jl 4b",
            // "call schedule",
            "mov rdi, 2",
            "call unblock",
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
            "mov r8, 0xffff8000000b8000",
            "mov byte ptr [r8], 0x32",
            "mov r8, 0",
            "6:",
            "add r8, 1",
            "cmp r8, 100000000",
            "jl 6b",
            // "call schedule",
            "call block",
            "jmp 5b",
            options(noreturn),
        );
    }
}
