use super::{
    pid::*,
    process::{Process, State},
    SCHEDULER_LOCK,
};
use crate::{
    arch::{x86_64::gdt, EntryFlags, P4Table},
    mem::{frame::Frame, PhysicalAddress, VirtualAddress, PAGE_SIZE},
    HIGHER_HALF,
};
use alloc::vec::Vec;
use core::ptr::addr_of;
use log::info;

const KERNEL_STACK_SIZE: usize = PAGE_SIZE as usize;
const USER_STACK_SIZE: usize = PAGE_SIZE as usize;

// naked function as a normal function allocates stack space during prologue
// as a result, the stack top doesn't exactly match up with the expected structure if a normal function is used
#[naked]
extern "C" fn user_task_init() {
    unsafe {
        core::arch::asm!(
            // SAFETY: the only way to get here is when `task_switch` transfers control to this new task
            // `SCHEDULER_LOCK` is locked before `task_switch` is run and is not unlocked before next statement
            "call scheduler_unlock",
            // jump to userspace
            "iretq",
            options(noreturn)
        );
    }
}

// TODO: should this be a naked function?
extern "C" fn kernel_task_init() {
    // TODO: add initialisation steps
    // for now, just returns
    // which transfers control to the user defined task function
    // This is possible due to the stack structure
    // `task_init` address is placed directly above the address of the user defined task function

    // SAFETY: the only way to get here is when `task_switch` transfers control to this new task
    // `SCHEDULER_LOCK` is locked before `task_switch` is run and is not unlocked before next statement
    unsafe {
        SCHEDULER_LOCK.unlock();
    }
}

// returns stack top and stack bottom
// can calculate one if you know the other and stack size requested
fn create_stack(
    size: usize,
    task_code: Option<(VirtualAddress, VirtualAddress, bool)>, // (code start, stack top, is_user_task)
) -> (PhysicalAddress, PhysicalAddress) {
    let stack = vec![0u8; size];
    let stack_bottom = stack.as_ptr();
    let mut stack_top = unsafe { stack_bottom.byte_add(size) };
    // 16 byte align
    stack_top = (stack_top as u64 & !0xf) as *const u8;

    if let Some((task_code_start, task_stack_top, is_user_task)) = task_code {
        unsafe {
            stack_top = stack_top.byte_sub(32);

            if is_user_task {
                info!("[user task] creating user task");
                // prepare to jump to userspace
                // add things to stack that `iretq` expects

                // stack segment offset in GDT
                let ds = gdt::get_user_data_segment_selector() as u64;
                core::ptr::write(stack_top as *mut u64, ds);
                info!("[user task] ds: {:#x} @ {:#x?}", ds, stack_top);
                stack_top = stack_top.byte_sub(8);

                // stack top
                // rsp value should belong to the user program's page table (not a higher half address)
                // 16 byte aligned
                // subtract 16 bytes to ensure that we are in the paged memory and not on the border address that is not mapped
                let user_task_stack_top = task_stack_top.to_inner() & !0xf - 16;
                core::ptr::write(stack_top as *mut u64, user_task_stack_top);
                info!(
                    "[user task] stack top: {:#x} @ {:#x?}",
                    user_task_stack_top, stack_top
                );
                stack_top = stack_top.byte_sub(8);

                // rflags (only interrupt bit set)
                let rflags = 0x200;
                core::ptr::write(stack_top as *mut u64, rflags);
                info!("[user task] rflags: {:#x} @ {:#x?}", rflags, stack_top);
                stack_top = stack_top.byte_sub(8);

                // code segment offset in GDT
                let cs = gdt::get_user_code_segment_selector() as u64;
                core::ptr::write(stack_top as *mut u64, cs);
                info!("[user task] cs: {:#x} @ {:#x?}", cs, stack_top);
                stack_top = stack_top.byte_sub(8);

                info!(
                    "[user task] code start: {:#x} @ {:#x?}",
                    task_code_start.to_inner(),
                    stack_top
                );
                info!("[use task] end creating user task");
            }

            core::ptr::write(stack_top as *mut u64, task_code_start.to_inner());

            stack_top = stack_top.byte_sub(8);
            let task_init = if is_user_task {
                user_task_init
            } else {
                kernel_task_init
            };
            core::ptr::write(stack_top as *mut u64, task_init as *const () as u64);

            // matching the initial stack with what the `task_switch` expects to see
            // callee saved registers - rbp, rbx, r12, r13, r14, r15
            for _ in 0..6 {
                stack_top = stack_top.byte_sub(8);
                core::ptr::write(stack_top as *mut u64, 0);
            }
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
pub(super) fn create_user_task(task: *const ()) -> Process {
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
    let us_stack_virt_base = VirtualAddress::new(0x800000);
    let (user_stack_bottom, user_stack_top) = create_stack(
        USER_STACK_SIZE,
        Some((
            us_task_code_virt_start,
            us_stack_virt_base.offset(USER_STACK_SIZE as u64),
            true,
        )),
    );
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

pub(super) fn create_kernel_task(task: *const ()) -> Process {
    let (task_code_start, task_code_end) = (
        VirtualAddress::new(task as u64),
        VirtualAddress::new(task as u64 + PAGE_SIZE),
    );

    let (_, kernel_stack_top) = create_stack(KERNEL_STACK_SIZE, None);
    let kernel_stack_top = unsafe { kernel_stack_top.to_virt().unwrap() };

    let (_, user_stack_top) = create_stack(
        USER_STACK_SIZE,
        Some((task_code_start, kernel_stack_top, false)),
    );
    let user_stack_top = unsafe { user_stack_top.to_virt().unwrap() };

    let cr3: u64;
    unsafe {
        core::arch::asm!(
            "mov {cr3}, cr3",
            cr3 = out(reg) cr3,
        );
    }

    Process {
        stack_top: user_stack_top,
        kernel_stack_top,
        cr3: PhysicalAddress::new(cr3),
        id: get_new_pid(),
        state: State::Ready,
    }
}
