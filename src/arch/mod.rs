#[cfg(target_arch="x86_64")]
mod x86_64;
#[cfg(target_arch="x86_64")]
pub(crate) use x86_64::{ init, _print };
