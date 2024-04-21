use super::{
    paging::{entry::EntryFlags, P4Table},
    ACTIVE_PAGETABLE,
};
use crate::{
    arch::x86_64::gdt,
    mem::{frame::Frame, PhysicalAddress, VirtualAddress, PAGE_SIZE},
    HIGHER_HALF,
};
use alloc::vec::Vec;
use core::ptr::addr_of;

#[naked]
unsafe extern "C" fn prog() {
    core::arch::asm!(
        "2:",
        "nop",
        "nop",
        "nop",
        "int 0x2e",
        "nop",
        "nop",
        "nop",
        "jmp 2b",
        options(noreturn)
    );
}

unsafe fn jump_to_userspace(code: VirtualAddress, stack_top: VirtualAddress) {
    let cs = gdt::get_user_code_segment_selector();
    let ds = gdt::get_user_data_segment_selector();

    core::arch::asm!(
        "push rax",   // stack segment
        "push rsi",   // rsp
        "push 0x200", // rflags (only interrupt bit set)
        "push rdx",   // code segment
        "push rdi",   // ret to virtual addr
        "iretq",
        in("rdi") code.to_inner(),
        in("rsi") stack_top.to_inner(),
        in("dx") cs,
        in("ax") ds,
    );
}

pub(super) fn run_userpace_code() {
    // create a new page table for the new task
    let mut task_page_table = unsafe { P4Table::with_kernel_mapped_to_higher_half() };

    // map task code in the page table
    let (task_code_start, task_code_end) = load_task_code();
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
    let (kernel_stack_bottom, kernel_stack_top) = create_stack(PAGE_SIZE as usize);
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

    // allocate a userspace stack for the task and map it in the page table
    let (user_stack_bottom, user_stack_top) = create_stack(PAGE_SIZE as usize);
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

    let old_page_table = ACTIVE_PAGETABLE.lock().switch(task_page_table);
    update_tss_rsp0(kernel_stack_top);

    unsafe {
        jump_to_userspace(us_task_code_virt_start, us_stack_virt_base);
    }

    // TODO: free resources, eg: memory used for stacks, page table
    ACTIVE_PAGETABLE.lock().switch(old_page_table);
}

fn update_tss_rsp0(rsp0_stack_top: PhysicalAddress) {
    let tss = unsafe { gdt::get_tss_mut() };

    let rsp0_stack_top = unsafe { rsp0_stack_top.to_virt().unwrap().to_inner() };
    tss.rsp0_low = rsp0_stack_top as u32;
    tss.rsp0_high = (rsp0_stack_top >> 32) as u32;

    // TODO: do I need to run ltr here for these changes to be picked up?
}

// returns stack top and stack bottom
// can calculate one if you know the other and stack size requested
fn create_stack(size: usize) -> (PhysicalAddress, PhysicalAddress) {
    let stack = vec![0u8; size];
    let stack_bottom = stack.as_ptr();
    let stack_top = unsafe { stack_bottom.byte_add(size) };
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

fn load_task_code() -> (PhysicalAddress, PhysicalAddress) {
    let start =
        PhysicalAddress::new(prog as *const () as u64 - unsafe { addr_of!(HIGHER_HALF) } as u64);
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
