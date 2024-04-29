use super::{
    pid::Pid,
    process::{Process, State},
};
use crate::{
    arch::x86_64::gdt,
    locks::SpinLock,
};
use alloc::sync::Arc;
use hashbrown::HashMap;
use log::trace;

#[derive(Debug)]
pub(super) struct Scheduler {
    // processes: BTreeMap<Pid, Arc<Process>>,
    // processes: IndexMap<Pid, Arc<Process>>,
    pub(super) processes: HashMap<Pid, Arc<SpinLock<Process>>>,
    pub(super) cur_proc: Pid,
    pub(super) ready_to_run: usize,
}

impl Scheduler {
    pub(super) fn new(init: Process) -> Self {
        let pid = init.id;
        // let mut processes = BTreeMap::new();
        let mut processes = HashMap::new();
        // let mut processes = IndexMap::new();
        processes.insert(pid, Arc::new(SpinLock::new(init)));

        Self {
            processes,
            cur_proc: pid,
            ready_to_run: 0,
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
                .or(self.processes.iter().filter(|(_, task)| task.lock().state == State::Ready).next()) // TODO: wraparound
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
}
