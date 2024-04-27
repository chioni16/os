mod pid;

use super::gdt::get_tss;
use crate::{
    arch::{get_cur_page_table_start, EntryFlags, P4Table},
    mem::{frame::Frame, PhysicalAddress, VirtualAddress, PAGE_SIZE},
    HIGHER_HALF,
};
use alloc::{boxed::Box, collections::BTreeMap, sync::Arc, vec::Vec};
use core::{mem::MaybeUninit, ptr::addr_of};
use log::{info, trace};
use pid::*;

const KERNEL_STACK_SIZE: usize = PAGE_SIZE as usize;
const USER_STACK_SIZE: usize = PAGE_SIZE as usize;

#[derive(Debug, Clone, Copy)]
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

pub(super) struct Scheduler {
    processes: BTreeMap<Pid, Arc<Process>>,
    cur_proc: Pid,
}

impl Scheduler {
    fn new(init: Process) -> Self {
        let pid = init.id;
        let mut processes = BTreeMap::new();
        processes.insert(pid, Arc::new(init));

        Self {
            processes,
            cur_proc: pid,
        }
    }

    fn add(&mut self, proc: Process) {
        let pid = proc.id;
        self.processes.insert(pid, Arc::new(proc));
    }

    fn schedule(&mut self) {
        info!("inside schedule2");
        let next_pid = {
            let mut iter = self.processes.keys();
            info!("inside schedule3");
            // find stops when it finds the entry (short circuits)
            let _ = iter.find(|pid| &self.cur_proc == *pid);
            info!("inside schedule4");
            iter.next().or(self.processes.keys().next())
        };
        info!("inside schedule3");
        if let Some(next_pid) = next_pid
            && *next_pid != self.cur_proc
        {
            let new_task = self.processes.get(next_pid).unwrap();
            let new_task = Arc::as_ptr(new_task) as u64;
            let old_task = self.processes.get(&self.cur_proc).unwrap();
            let old_task = Arc::as_ptr(old_task) as u64;
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
    info!("inside schedule1");
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

    let mut scheduler = Scheduler::new(init);

    let p1 = create_task(func1 as _);
    info!("p1: {:#x?}", p1);
    scheduler.add(p1);
    let p2 = create_task(func2 as _);
    info!("p2: {:#x?}", p2);
    scheduler.add(p2);

    unsafe {
        SCHEDULER.write(scheduler);
    }
    info!("Scheduler started");
    loop {
        unsafe { core::arch::asm!("cli"); }
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

// #[naked]
extern "C" fn func1() {
    unsafe {
        core::arch::asm!(
            "3:",
            "mov r8, 0xffff8000000b8000",
            "mov byte ptr [r8], 0x31",
            "call schedule",
            "jmp 3b",
        );
    }
}

fn func2() {
    unsafe {
        core::arch::asm!(
            "4:",
            "mov r8, 0xffff8000000b8000",
            "mov byte ptr [r8], 0x32",
            "call schedule",
            "jmp 4b",
        );
    }
}

// TODO: should this be a naked function?
fn task_init() {
    // TODO: add initialisation steps
    // for now, just returns
    // which transfers control to the user defined task function
    // This is possible due to the stack structure
    // `task_init` address is placed directly above the address of the user defined task function
}

// returns stack top and stack bottom
// can calculate one if you know the other and stack size requested
fn create_stack(
    size: usize,
    task_code: Option<VirtualAddress>,
) -> (PhysicalAddress, PhysicalAddress) {
    let stack = vec![0u8; size];
    let stack_bottom = stack.as_ptr();
    let mut stack_top = unsafe { stack_bottom.byte_add(size) };
    // 16 byte align
    stack_top = (stack_top as u64 & !0xf) as *const u8;

    if let Some(task_code) = task_code {
        unsafe {
            stack_top = stack_top.byte_sub(32);
            core::ptr::write(stack_top as *mut u64, task_code.to_inner());

            stack_top = stack_top.byte_sub(8);
            core::ptr::write(stack_top as *mut u64, task_init as *const () as u64);

            // matching the initial stack with what the `task_switch` expects to see
            stack_top = stack_top.byte_sub(6 * 8);
        }
    }

    core::mem::forget(stack);
    (
        PhysicalAddress::new(stack_bottom as u64 - unsafe { addr_of!(HIGHER_HALF) } as u64),
        PhysicalAddress::new(stack_top as u64 - unsafe { addr_of!(HIGHER_HALF) } as u64),
    )
}

// SAFETY:
// ensure that the stack_top points to the top (address of the higher end of the stack)
// of a valid stack of size `size`
unsafe fn free_stack(stack_bottom: *const u8, stack_top: *const u8) {
    let stack_size = (stack_top as u64 - stack_bottom as u64) as usize;
    let stack = Vec::from_raw_parts(stack_bottom as *mut u8, stack_size, stack_size);
    drop(stack);
}

fn load_task_code(task: *const ()) -> (PhysicalAddress, PhysicalAddress) {
    let start = PhysicalAddress::new(task as u64 - unsafe { addr_of!(HIGHER_HALF) } as u64);
    let end = start.offset(PAGE_SIZE);
    (start, end)
}

fn map_task_page_table(
    table: &mut P4Table,
    virt_addr_range_base: VirtualAddress,
    frames: impl Iterator<Item = Frame>,
    flags_to_set: EntryFlags,
) {
    for (i, frame) in frames.enumerate() {
        let i = i as u64;

        table.map_4KiB(
            virt_addr_range_base.offset(i * PAGE_SIZE),
            frame.start_address(),
            flags_to_set,
        );
    }
}

// fn create_task(task: *const ()) -> (P4Table, VirtualAddress, VirtualAddress, VirtualAddress) {
fn create_task(task: *const ()) -> Process {
    let (task_code_start, task_code_end) = load_task_code(task);

    // create a new page table for the new task
    let mut task_page_table = unsafe { P4Table::with_kernel_mapped_to_higher_half() };

    // map task code in the page table
    let us_code_virt_base = VirtualAddress::new(0x400000);
    map_task_page_table(
        &mut task_page_table,
        us_code_virt_base,
        Frame::range_inclusive(
            &Frame::containing_address(task_code_start),
            &Frame::containing_address(task_code_end),
        ),
        EntryFlags::USER_ACCESSIBLE | EntryFlags::PRESENT,
    );
    // calculate jump address
    let us_task_code_virt_start = {
        let task_code_start_frame = Frame::containing_address(task_code_start);
        let code_start_page_offset =
            task_code_start.to_inner() - task_code_start_frame.start_address().to_inner();
        us_code_virt_base.offset(code_start_page_offset)
    };

    // allocate a kernel stack for the task and map it in the page table
    let (kernel_stack_bottom, kernel_stack_top) = create_stack(KERNEL_STACK_SIZE, None);
    // TODO: should I map kernel stack in the original kernel page table and then copy it into the new prog page table?
    let kernel_stack_bottom_virt = unsafe { kernel_stack_bottom.to_virt().unwrap() };
    map_task_page_table(
        &mut task_page_table,
        kernel_stack_bottom_virt,
        Frame::range_inclusive(
            &Frame::containing_address(kernel_stack_bottom),
            &Frame::containing_address(kernel_stack_top),
        ),
        EntryFlags::PRESENT | EntryFlags::WRITABLE,
    );
    // Virtual address is of kernel page table
    let kernel_stack_top = unsafe { kernel_stack_top.to_virt().unwrap() };

    // allocate a userspace stack for the task and map it in the page table
    let (user_stack_bottom, user_stack_top) =
        create_stack(USER_STACK_SIZE, Some(us_task_code_virt_start));
    let us_stack_virt_base = VirtualAddress::new(0x800000);
    map_task_page_table(
        &mut task_page_table,
        us_stack_virt_base,
        Frame::range_inclusive(
            &Frame::containing_address(user_stack_bottom),
            &Frame::containing_address(user_stack_top),
        ),
        EntryFlags::USER_ACCESSIBLE | EntryFlags::PRESENT | EntryFlags::WRITABLE,
    );
    // virtual address is of user page table
    let user_stack_top = {
        let user_stack_size = user_stack_top.to_inner() - user_stack_bottom.to_inner();
        us_stack_virt_base.offset(user_stack_size)
    };

    // (
    //     task_page_table,
    //     us_task_code_virt_start,
    //     kernel_stack_top,
    //     user_stack_top,
    // );

    Process {
        stack_top: user_stack_top,
        kernel_stack_top,
        cr3: task_page_table.forget(),
        id: get_new_pid(),
        state: State::Ready,
    }
}
