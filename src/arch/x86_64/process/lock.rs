use crate::arch::{disable_interrupts, enable_interrupts};
use core::sync::atomic::{AtomicUsize, Ordering};

pub(super) struct Lock {
    irq_disable_counter: AtomicUsize,
}

impl Lock {
    pub(super) const fn new() -> Self {
        Self {
            irq_disable_counter: AtomicUsize::new(0),
        }
    }

    pub(super) fn lock(&self) {
        disable_interrupts();
        self.irq_disable_counter.fetch_add(1, Ordering::Relaxed);
    }

    // make sure that `unlock` is called only after locking the lock
    pub(super) unsafe fn unlock(&self) {
        let old = self.irq_disable_counter.fetch_sub(1, Ordering::Relaxed);
        if old == 1 {
            enable_interrupts();
        }
    }
}
