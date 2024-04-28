use core::sync::atomic::{AtomicU32, Ordering};

static PID_COUNTER: AtomicU32 = AtomicU32::new(0);

pub(super) fn get_new_pid() -> Pid {
    let pid = PID_COUNTER.fetch_add(1, Ordering::Relaxed);
    Pid(pid)
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub(super) struct Pid(u32);
