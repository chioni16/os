#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
pub(crate) use x86_64::{
    _print, disable_interrupts, enable_interrupts, init, is_int_enabled,
    map_rw_using_current_page_table, translate_using_current_page_table,
    unmap_rw_using_current_page_table, ACTIVE_PAGETABLE,
};
