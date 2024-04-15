use gimli::X86_64;

use super::RegisterSet;

pub fn unwind(register_set: RegisterSet) {
    let mut rbp = register_set.get(X86_64::RBP).unwrap();

    while rbp != 0 {
        rbp = unsafe {
            let ret = core::ptr::read_unaligned((rbp + 8) as *const u64);
            crate::print!("{:#x} ", ret);
            core::ptr::read_unaligned(rbp as *const u64)
        };
    }
}