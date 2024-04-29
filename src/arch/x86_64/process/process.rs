use super::pid::Pid;
use crate::mem::{PhysicalAddress, VirtualAddress};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum State {
    Ready,
    Running,
    Waiting,
}

// needed as we attempt to access the fields from inline assembly (`switch`)
#[repr(C, packed)]
#[derive(Debug)]
pub(super) struct Process {
    // the fields accessed by inline assembly are placed at the top (for easy offset calculation lol)

    // virtual address as per page table pointed to by the `cr3` field
    pub(super) stack_top: VirtualAddress,
    // used when privilege levels change from CPL3 to CPL0
    // stored in the TSS.RSP0 field
    pub(super) kernel_stack_top: VirtualAddress,
    pub(super) cr3: PhysicalAddress,

    pub(super) id: Pid,
    pub(super) state: State,
    // scheduling policy

    // statistics
}
