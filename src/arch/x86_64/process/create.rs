use super::{pid::*, Process, State};
use crate::{
    arch::{EntryFlags, P4Table},
    mem::{frame::Frame, PhysicalAddress, VirtualAddress, PAGE_SIZE},
    HIGHER_HALF,
};
use alloc::vec::Vec;
use core::ptr::addr_of;

const KERNEL_STACK_SIZE: usize = PAGE_SIZE as usize;
const USER_STACK_SIZE: usize = PAGE_SIZE as usize;

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

            // stack_top = stack_top.byte_sub(8);
            // core::ptr::write(stack_top as *mut u64, task_init as *const () as u64);

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
pub(super) fn create_task(task: *const ()) -> Process {
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

pub(super) fn create_kernel_task(task: *const ()) -> Process {
    let (task_code_start, task_code_end) = (
        VirtualAddress::new(task as u64),
        VirtualAddress::new(task as u64 + PAGE_SIZE),
    );

    let (_, kernel_stack_top) = create_stack(KERNEL_STACK_SIZE, None);
    let kernel_stack_top = unsafe { kernel_stack_top.to_virt().unwrap() };

    let (_, user_stack_top) = create_stack(USER_STACK_SIZE, Some(task_code_start));
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
