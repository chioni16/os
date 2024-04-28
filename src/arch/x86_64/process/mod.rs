mod create;
mod pid;

use super::gdt::get_tss;
use crate::locks::SpinLock;
use crate::{
    arch::get_cur_page_table_start,
    mem::{PhysicalAddress, VirtualAddress},
};
use alloc::sync::Arc;
use core::mem::MaybeUninit;
use create::create_kernel_task;
use log::{info, trace};
use pid::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Ready,
    Running,
    Waiting,
}

// needed as we attempt to access the fields from inline assembly (`switch`)
#[repr(C, packed)]
#[derive(Debug)]
pub(crate) struct Process {
    // the fields accessed by inline assembly are placed at the top (for easy offset calculation lol)

    // virtual address as per page table pointed to by the `cr3` field
    stack_top: VirtualAddress,
    // used when privilege levels change from CPL3 to CPL0
    // stored in the TSS.RSP0 field
    kernel_stack_top: VirtualAddress,
    cr3: PhysicalAddress,

    id: Pid,
    state: State,
    // scheduling policy

    // statistics
}

use hashbrown::HashMap;

#[derive(Debug)]
pub(super) struct Scheduler {
    // processes: BTreeMap<Pid, Arc<Process>>,
    // processes: IndexMap<Pid, Arc<Process>>,
    processes: HashMap<Pid, Arc<SpinLock<Process>>>,
    cur_proc: Pid,
}

impl Scheduler {
    fn new(init: Process) -> Self {
        let pid = init.id;
        // let mut processes = BTreeMap::new();
        let mut processes = HashMap::new();
        // let mut processes = IndexMap::new();
        processes.insert(pid, Arc::new(SpinLock::new(init)));

        Self {
            processes,
            cur_proc: pid,
        }
    }

    fn add(&mut self, proc: Process) {
        let pid = proc.id;
        self.processes.insert(pid, Arc::new(SpinLock::new(proc)));
    }

    #[inline(never)]
    fn schedule(&mut self) {
        unsafe {
            core::arch::asm!("cli");
        }

        let next_pid = {
            let mut iter = self.processes.iter();
            // find stops when it finds the entry (short circuits)
            let _ = iter.find(|(pid, _)| &self.cur_proc == *pid);
            iter.filter(|(_, task)| task.lock().state == State::Ready)
                .next()
                .or(self.processes.iter().next()) // TODO: wraparound
        };
        if let Some((next_pid, _)) = next_pid
            && *next_pid != self.cur_proc
        {
            trace!("changing {:x?} --> {:x?}", self.cur_proc, next_pid);
            let new_task = self.processes.get(next_pid).unwrap();
            let old_task = self.processes.get(&self.cur_proc).unwrap();
            new_task.lock().state = State::Running;
            old_task.lock().state = State::Ready;

            self.cur_proc = new_task.lock().id;

            // let new_task = Arc::as_ptr(new_task) as u64;
            // let old_task = Arc::as_ptr(old_task) as u64;
            let new_task = new_task.lock().get_val_addr() as u64;
            let old_task = old_task.lock().get_val_addr() as u64;
            unsafe {
                core::arch::asm!(
                    "call task_switch",
                    in("rdi") old_task,
                    in("rsi") new_task,
                    in("rdx") get_tss() as *const _ as u64,
                    clobber_abi("C")
                );
            }
        }
    }
}

static mut SCHEDULER: MaybeUninit<Scheduler> = MaybeUninit::uninit();

#[no_mangle]
fn schedule() {
    let scheduler = unsafe { SCHEDULER.assume_init_mut() };
    scheduler.schedule();
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
            "cmp r8, 1000000",
            "jl 4b",
            "call schedule",
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
            "cmp r8, 1000000",
            "jl 6b",
            "call schedule",
            "jmp 5b",
            options(noreturn),
        );
    }
}
