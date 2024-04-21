use log::info;

use crate::arch::x86_64::{rdmsr, wrmsr};

#[naked]
unsafe extern "C" fn syscall_handler() {
    // TODO: write a proper one
    // Does this need to be a naked function? Probably yes.
    core::arch::asm!("nop", "nop", "nop", "sysretq", options(noreturn),);
}

// SAFETY: assumes the presence of EFER MSR
unsafe fn enable_efer_syscall_extension() {
    const IA32_EFER_MSR: u32 = 0xC0000080;
    let value = rdmsr(IA32_EFER_MSR);
    // bit 0 - System Call Extensions
    wrmsr(IA32_EFER_MSR, value | 0x1);
}

// SAFETY: assumes presence of IA32_START MSR
unsafe fn init_star_msr() {
    // AMD manual Vol2 Ch6 System Instructions
    const IA32_STAR_MSR: u32 = 0xC0000081;
    let value = (0x8 << 32) // used for kernel code and stack/data segments ( 8 - kernel code segment, 8 + 8 - kernel stack segment)
                    | ((0x20  | 0x3) << 48); // used for user code and stack/data segments (32 - TSS2, 32 + 8 user stack segment, 32 + 16 - user code segment, or with 3 to represent RPL = 3 in segment selector)

    wrmsr(IA32_STAR_MSR, value);
}

// SAFETY: assumes presence of IA32_LSTART MSR
unsafe fn init_lstar_msr() {
    // AMD manual Vol2 Ch6 System Instructions
    const IA32_LSTAR_MSR: u32 = 0xC0000082;
    // TODO: what type should be given to the pointer to a naked function?
    // or does it not matter in the end as we cast it to u64 anyway?
    let value = syscall_handler as *const () as u64;

    wrmsr(IA32_LSTAR_MSR, value);
}

// SAFETY: assumes presence of IA32_FMASK MSR
unsafe fn init_fmask_msr() {
    // AMD manual Vol2 Ch6 System Instructions
    const IA32_FMASK_MSR: u32 = 0xC0000084;
    // we disable the interrupt flag when syscall occurs
    // we enable interrupts back on sysret
    // https://en.wikipedia.org/wiki/FLAGS_register
    let value = 1 << 9;

    wrmsr(IA32_FMASK_MSR, value);
}

pub(super) fn init() {
    // SAFETY: these MSRs have become fairly widespread
    // I will rethink this if I find a modern processor that doesn't support them
    // until then...
    unsafe {
        enable_efer_syscall_extension();
        init_star_msr();
        init_lstar_msr();
        // TODO: Do I need to disable interrupts on syscall?
        // remember, you have kernel stack per task model
        init_fmask_msr();
    }

    info!("Syscall mechanism initialised");
}
