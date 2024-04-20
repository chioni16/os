use log::trace;

use crate::mem::{frame::Frame, PhysicalAddress};

use super::{EntryFlags, ACTIVE_PAGETABLE};

// #[derive(Debug)]
// pub(crate) struct Mmio {
//     start: Frame,
//     end: Frame,
// }

// impl Mmio {
//     pub(crate) fn new(start: PhysicalAddress, end: PhysicalAddress) -> Self {
//         trace!("MMIO mapping...");
//         let mut guard = ACTIVE_PAGETABLE.lock();

//         assert!(guard.translate(unsafe { start.to_virt().unwrap() } ).is_none());

//         let start = Frame::containing_address(start);
//         let end = Frame::containing_address(end);

//         for frame in Frame::range_inclusive(&start, &end) {
//             let phys_addr = frame.start_address();
//             let virt_addr = unsafe { phys_addr.to_virt().unwrap() };
//             guard.map_4KiB(
//                 virt_addr,
//                 phys_addr,
//                 EntryFlags::PRESENT
//                     | EntryFlags::WRITABLE
//                     | EntryFlags::WRITE_THROUGH
//                     | EntryFlags::NO_CACHE,
//             );
//         }

//         Self { start, end }
//     }
// }

// impl Drop for Mmio {
//     fn drop(&mut self) {
//         let mut guard = ACTIVE_PAGETABLE.lock();
//         for frame in Frame::range_inclusive(&self.start, &self.end) {
//             let phys_addr = frame.start_address();
//             let virt_addr = unsafe { phys_addr.to_virt().unwrap() };
//             guard.unmap(virt_addr);
//         }
//     }
// }

pub(crate) fn map(start: PhysicalAddress, end: PhysicalAddress) {
    trace!("MMIO mapping...");
    let mut guard = ACTIVE_PAGETABLE.lock();

    // if there's already a mapping for the address, we are gonna assume that this range has been mapped
    // A fair assumption as this function will be used to map the MMIO devices
    // No two devices will be mapped to address ranges that share the same start physical address
    if guard
        .translate(unsafe { start.to_virt().unwrap() })
        .is_some()
    {
        return;
    }

    let start = Frame::containing_address(start);
    let end = Frame::containing_address(end);

    for frame in Frame::range_inclusive(&start, &end) {
        let phys_addr = frame.start_address();
        let virt_addr = unsafe { phys_addr.to_virt().unwrap() };
        guard.map_4KiB(
            virt_addr,
            phys_addr,
            EntryFlags::PRESENT
                | EntryFlags::WRITABLE
                | EntryFlags::WRITE_THROUGH
                | EntryFlags::NO_CACHE,
        );
    }
}
