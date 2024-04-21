use alloc::boxed::Box;
use core::ptr::{addr_of, addr_of_mut};
use log::{info, trace};

#[derive(Debug)]
#[repr(C, packed)]
pub(super) struct Gdt {
    null: u64,
    code: u64,
    data: u64,
    tss1: u64,
    tss2: u64,
    user_data: u64,
    user_code: u64,
}

#[derive(Debug, Default)]
#[repr(C, packed)]
pub(super) struct TaskStateSegment {
    reserved0: u32,
    pub(super) rsp0_low: u32,
    pub(super) rsp0_high: u32,
    pub(super) rsp1_low: u32,
    pub(super) rsp1_high: u32,
    pub(super) rsp2_low: u32,
    pub(super) rsp2_high: u32,
    reserved1: u64,
    pub(super) ist1_low: u32,
    pub(super) ist1_high: u32,
    pub(super) ist2_low: u32,
    pub(super) ist2_high: u32,
    pub(super) ist3_low: u32,
    pub(super) ist3_high: u32,
    pub(super) ist4_low: u32,
    pub(super) ist4_high: u32,
    pub(super) ist5_low: u32,
    pub(super) ist5_high: u32,
    pub(super) ist6_low: u32,
    pub(super) ist6_high: u32,
    pub(super) ist7_low: u32,
    pub(super) ist7_high: u32,
    reserved2: u64,
    reserved3: u16,
    pub(super) iopb: u16,
}

fn tss_init() {
    // SAFETY: the `gdt64` structure has 'static lifetime
    let gdt64_addr = unsafe { addr_of_mut!(crate::gdt64) };
    let gdt64 = unsafe { &mut *(gdt64_addr as *mut Gdt) };

    // heap alloc new TSS for the core
    let tss = Box::new(TaskStateSegment::default());
    // don't deallocate the memory
    let tss_addr = Box::into_raw(tss) as u64;
    trace!("TSS address: {:#x?}", tss_addr);
    trace!("TSS size: {:#x?}", core::mem::size_of::<TaskStateSegment>());

    // update the tss1 and tss2 fields
    let tss_addr_00_15 = (tss_addr >> 0) as u16;
    let tss_addr_16_23 = (tss_addr >> 16) as u8;
    let tss_addr_24_31 = (tss_addr >> 24) as u8;
    let tss_addr_32_63 = (tss_addr >> 32) as u32;

    let tss_size = core::mem::size_of::<TaskStateSegment>();
    let tss_size_00_15 = (tss_size >> 0) as u16;
    let tss_size_16_19 = ((tss_size >> 16) as u8) & 0x0f;

    let access_byte: u8 = (1 << 7) | (1 << 3) | (1 << 0); // present, executable (why?), accessed (without accessed doesn't work)
    let flags: u8 = 1 << 1;

    gdt64.tss1 = (tss_size_00_15 as u64) << 0
        | (tss_addr_00_15 as u64) << 16
        | (tss_addr_16_23 as u64) << 32
        | (access_byte as u64) << 40
        | (tss_size_16_19 as u64) << 48
        | (flags as u64) << 52
        | (tss_addr_24_31 as u64) << 56;
    gdt64.tss2 = tss_addr_32_63 as u64;

    trace!("gdt64: {:#x?}", gdt64);

    // load tss (ltr)
    let tss_field_addr = addr_of!(gdt64.tss1);
    let tss_field_offset = tss_field_addr as u64 - gdt64_addr as u64;
    trace!("gdt addr: {:#x?}", gdt64_addr);
    trace!("tss field addr: {:#x?}", tss_field_addr);
    trace!("gdt offset: {:#x}", tss_field_offset);
    unsafe {
        core::arch::asm!(
            "ltr ax",
            in("ax") tss_field_offset as u16,
            options(nostack),
        );
    }
}

pub(super) fn init() {
    tss_init();
    info!("GDT initialised");
}

fn get_gdt() -> &'static Gdt {
    // SAFETY: the `gdt64` structure has 'static lifetime
    let gdt64_addr = unsafe { addr_of_mut!(crate::gdt64) };
    let gdt64 = unsafe { &mut *(gdt64_addr as *mut Gdt) };
    gdt64
}

// SAFETY: TSS for the core should be initialised before calling this function
unsafe fn get_tss_addr(gdt64: &Gdt) -> u64 {
    let tss_addr_high = gdt64.tss2 << 32;
    let tss_addr_low = (gdt64.tss1 >> 16) & 0xffffff | (gdt64.tss1 >> 56) << 24;
    let tss_addr = tss_addr_high | tss_addr_low;

    tss_addr
}

// SAFETY: TSS for the core should be initialised before calling this function
pub(super) unsafe fn get_tss() -> &'static TaskStateSegment {
    let gdt64 = get_gdt();

    let tss_addr = get_tss_addr(gdt64);
    &*(tss_addr as *const TaskStateSegment)
}

// SAFETY: TSS for the core should be initialised before calling this function
pub(super) unsafe fn get_tss_mut() -> &'static mut TaskStateSegment {
    let gdt64 = get_gdt();

    let tss_addr = get_tss_addr(gdt64);
    &mut *(tss_addr as *mut TaskStateSegment)
}

pub(super) fn get_kernel_code_segment_selector() -> u16 {
    let gdt64 = get_gdt();

    let gdt64_addr = gdt64 as *const _ as u64;
    let kernel_code_addr = addr_of!(gdt64.code) as u64;

    (kernel_code_addr - gdt64_addr) as u16 | 0b000 // DPL = 0, descriptor belongs to GDT
}

pub(super) fn get_kernel_data_segment_selector() -> u16 {
    let gdt64 = get_gdt();

    let gdt64_addr = gdt64 as *const _ as u64;
    let kernel_data_addr = addr_of!(gdt64.data) as u64;

    (kernel_data_addr - gdt64_addr) as u16 | 0b000 // DPL = 0, descriptor belongs to GDT
}

pub(super) fn get_user_data_segment_selector() -> u16 {
    let gdt64 = get_gdt();

    let gdt64_addr = gdt64 as *const _ as u64;
    let user_data_addr = addr_of!(gdt64.user_data) as u64;

    (user_data_addr - gdt64_addr) as u16 | 0b011 // DPL = 3, descriptor belongs to GDT
}

pub(super) fn get_user_code_segment_selector() -> u16 {
    let gdt64 = get_gdt();

    let gdt64_addr = gdt64 as *const _ as u64;
    let user_code_addr = addr_of!(gdt64.user_code) as u64;

    (user_code_addr - gdt64_addr) as u16 | 0b011 // DPL = 3, descriptor belongs to GDT
}
