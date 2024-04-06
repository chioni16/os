#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
pub(crate) use x86_64::{
    _print, disable_interrupts, enable_interrupts, init, is_int_enabled,
    translate_using_current_page_table,
};
