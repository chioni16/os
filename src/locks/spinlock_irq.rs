use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::{disable_interrupts, enable_interrupts, is_int_enabled};

// Interrupts safe version of SpinLock<T>
// To be used to lock the resources that are modifed/read from within an interrupt handler
// Locking disables interrupts, which are then restored once the Guard goes out of scope
#[derive(Debug)]
pub struct SpinLockIrq<T> {
    locked: AtomicBool,
    // shows if the interrupts were enabled at the moment when the lock is taken
    interrupts: AtomicBool,
    value: UnsafeCell<T>,
}

impl<T> SpinLockIrq<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            interrupts: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> GuardIrq<T> {
        // I hope the Relaxed ordering suffices here
        // as I think we can piggy back on the causal relationships formed by the Acquire-Release semantics of `locked` field.

        // It's okay to attempt to disable interrupts before taking the lock, even if we end up waiting for the lock to be taken (spin)
        // This is because waiting indicates that someone else has already taken the lock, the prerequisite for which is disabling the interrupts.
        // In this case, the `interrupts_enabled` comes out `false` and the disable code is not run.

        let interrupts_enabled = is_int_enabled();
        self.interrupts.store(interrupts_enabled, Ordering::Relaxed);
        if interrupts_enabled {
            disable_interrupts();
        }

        while self.locked.swap(true, Ordering::Acquire) {
            core::hint::spin_loop();
        }
        GuardIrq { lock: self }
    }
}

unsafe impl<T> Sync for SpinLockIrq<T> where T: Send {}

pub struct GuardIrq<'a, T> {
    lock: &'a SpinLockIrq<T>,
}

impl<T> Deref for GuardIrq<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for GuardIrq<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T> Drop for GuardIrq<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
        if self.lock.interrupts.load(Ordering::Relaxed) {
            enable_interrupts();
        }
    }
}
