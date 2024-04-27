#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
pub(crate) use x86_64::{
    _print, disable_interrupts, enable_interrupts, get_cur_page_table_start, init, is_int_enabled,
    EntryFlags, P4Table, ACTIVE_PAGETABLE,
};
