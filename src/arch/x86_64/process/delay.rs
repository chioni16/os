use alloc::{collections::BinaryHeap, sync::Arc, vec::Vec};
use core::cmp::Reverse;
use log::{info, trace};

use crate::arch::x86_64::timers::hpet::Hpet;

use super::pid::Pid;

#[derive(Debug)]
struct Delay(Pid, Reverse<u64>);

impl PartialEq for Delay {
    fn eq(&self, other: &Self) -> bool {
        self.1.eq(&other.1)
    }
}

impl Eq for Delay {}

impl PartialOrd for Delay {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.1.partial_cmp(&other.1)
    }
}

impl Ord for Delay {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.1.cmp(&other.1)
    }
}

#[derive(Debug)]
pub(super) struct Delays {
    heap: BinaryHeap<Delay>,
    hpet: Arc<Hpet>,
}

impl Delays {
    pub(super) fn new(hpet: Arc<Hpet>) -> Self {
        Self {
            heap: BinaryHeap::new(),
            hpet,
        }
    }

    pub(super) fn add(&mut self, pid: Pid, delay_ns: u64) {
        let time_since_boot = self.hpet.time_since_boot_in_ns();
        let delay = Delay(pid, Reverse(time_since_boot + delay_ns));
        trace!("[add] time_since_boot: {:?}", time_since_boot);
        trace!("[add] added delay: {:?}", delay);
        self.heap.push(delay)
    }

    pub(super) fn get_expired_timers(&mut self) -> Vec<Pid> {
        let time_since_boot = self.hpet.time_since_boot_in_ns();

        let mut expired_delays = vec![];
        while let Some(Delay(pid, Reverse(delay))) = self.heap.peek()
            && time_since_boot > *delay
        {
            expired_delays.push(*pid);
            self.heap.pop();
        }

        trace!("[expire] time_since_boot: {:?}", time_since_boot);
        trace!("[expire] expired_delays: {:?}", expired_delays);

        expired_delays
    }

    pub(super) fn get_smallest_delay(&self) -> Option<u64> {
        self.heap.peek().map(|Delay(_, Reverse(delay))| {
            let time_since_boot = self.hpet.time_since_boot_in_ns();
            delay.saturating_sub(time_since_boot)
        })
    }
}
