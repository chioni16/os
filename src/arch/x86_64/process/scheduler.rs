use super::{
    delay::Delays,
    pid::Pid,
    process::{Process, State},
};
use crate::{
    arch::x86_64::{apic::lapic::get_lapic, gdt, timers::hpet::Hpet},
    locks::SpinLock,
};
use alloc::sync::Arc;
use hashbrown::HashMap;
use log::{info, trace};

// 100 ms in ns
const SCHEDULER_FREQ: u32 = 10u32.pow(8);

#[derive(Debug)]
pub(super) struct Scheduler {
    // processes: BTreeMap<Pid, Arc<Process>>,
    // processes: IndexMap<Pid, Arc<Process>>,
    pub(super) processes: HashMap<Pid, Arc<SpinLock<Process>>>,
    pub(super) cur_proc: Pid,
    pub(super) ready_to_run: usize,
    pub(super) delays: Delays,
}

impl Scheduler {
    pub(super) fn new(init: Process, hpet: Arc<Hpet>) -> Self {
        let pid = init.id;
        // let mut processes = BTreeMap::new();
        let mut processes = HashMap::new();
        // let mut processes = IndexMap::new();
        processes.insert(pid, Arc::new(SpinLock::new(init)));

        Self {
            processes,
            cur_proc: pid,
            ready_to_run: 0,
            delays: Delays::new(hpet),
        }
    }

    pub(super) fn add(&mut self, proc: Process) {
        let pid = proc.id;
        assert_eq!(proc.state, State::Ready);
        self.ready_to_run += 1;
        self.processes.insert(pid, Arc::new(SpinLock::new(proc)));
    }

    // SAFETY: should be called only one interrupts are disabled
    #[inline(never)]
    pub(super) unsafe fn schedule(&mut self) {
        let next_pid = {
            let mut iter = self.processes.iter();
            // find stops when it finds the entry (short circuits)
            let _ = iter.find(|(pid, _)| &self.cur_proc == *pid);
            iter.filter(|(_, task)| task.lock().state == State::Ready)
                .next()
                .or(self
                    .processes
                    .iter()
                    .filter(|(_, task)| task.lock().state == State::Ready)
                    .next()) // TODO: wraparound
        };
        if let Some((next_pid, _)) = next_pid
            && *next_pid != self.cur_proc
        {
            trace!("changing {:x?} --> {:x?}", self.cur_proc, next_pid);

            let old_task = {
                let mut old_task = self.processes.get(&self.cur_proc).unwrap().lock();

                if old_task.state == State::Running {
                    old_task.state = State::Ready;
                }

                old_task.get_val_addr() as u64
            };

            let new_task = {
                let mut new_task = self.processes.get(next_pid).unwrap().lock();

                new_task.state = State::Running;

                self.cur_proc = new_task.id;

                new_task.get_val_addr() as u64
            };

            // set scheduler wakeup interrupt
            let int_delay = self
                .delays
                .get_smallest_delay()
                .map(|delay| core::cmp::min(delay, SCHEDULER_FREQ as u64) as u32)
                .unwrap_or(SCHEDULER_FREQ);

            // WARNING: avoid setting int_delay to 0
            // add some buffer so that `task_switch` can complete
            // TODO: Understand the underlying problem better. How to better deal with this?
            // maybe I can remove the expired timers
            // but that's just pushing the problem one step further
            // during the time required to remove expired timers, other timers can expire

            let int_delay = core::cmp::max(int_delay, 1000);
            trace!("[schedule] int_delay: {int_delay}");

            // SAFETY: lapic initialised by the time we get here
            unsafe {
                get_lapic().set_timer_initial_count_in_ns(int_delay);
            }

            unsafe {
                core::arch::asm!(
                    "call task_switch",
                    in("rdi") old_task,
                    in("rsi") new_task,
                    in("rdx") gdt::get_tss() as *const _ as u64,
                    clobber_abi("C")
                );
            }
        }
    }

    pub(super) fn block_current(&mut self) {
        let pid = &self.cur_proc;
        let task = self.processes.get(pid).unwrap();
        task.lock().state = State::Waiting;
        self.ready_to_run -= 1;
    }

    pub(super) fn unblock(&mut self, pid: Pid) {
        let mut task = self.processes.get(&pid).unwrap().lock();
        task.state = State::Ready;

        self.ready_to_run += 1;
    }
}
