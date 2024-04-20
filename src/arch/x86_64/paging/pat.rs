use alloc::vec::Vec;

use crate::arch::x86_64::{rdmsr, wrmsr};
use core::arch::x86_64::{CpuidResult, __cpuid};

use super::table::tlb_flush_all;

pub(super) fn supports_pat() -> bool {
    let CpuidResult { edx, .. } = unsafe { __cpuid(1) };
    edx & (1 << 16) != 0
}

// SAFETY: only run if the processor supports PAT (use cpuid to get this)
pub(super) unsafe fn read_pat_msr() -> PageAttributes {
    let msr = unsafe { rdmsr(0x277) };
    msr.into()
}

// SAFETY: inavlid values can cause GP
// top 5 bits of each byte should be 0
// need to be consistent across all the processors
pub(super) unsafe fn write_pat_msr(pas: PageAttributes) {
    unsafe {
        wrmsr(0x277, pas.into());
        tlb_flush_all();
    }
}

#[derive(Debug)]
pub(super) enum MemoryType {
    Uncacheable,
    WriteCombining,
    WriteThrough,
    WriteProtected,
    WriteBack,
    Uncached,
    Reserved(u8),
}

impl From<u8> for MemoryType {
    fn from(value: u8) -> Self {
        match value {
            0x0 => MemoryType::Uncacheable,
            0x1 => MemoryType::WriteCombining,
            0x4 => MemoryType::WriteThrough,
            0x5 => MemoryType::WriteProtected,
            0x6 => MemoryType::WriteBack,
            0x7 => MemoryType::Uncached,
            o => MemoryType::Reserved(o),
        }
    }
}

impl From<MemoryType> for u8 {
    fn from(value: MemoryType) -> Self {
        match value {
            MemoryType::Uncacheable => 0x0,
            MemoryType::WriteCombining => 0x1,
            MemoryType::WriteThrough => 0x4,
            MemoryType::WriteProtected => 0x5,
            MemoryType::WriteBack => 0x6,
            MemoryType::Uncached => 0x7,
            MemoryType::Reserved(o) => o,
        }
    }
}

#[derive(Debug)]
pub(super) struct PageAttributes(pub(super) [MemoryType; 8]);

impl From<u64> for PageAttributes {
    fn from(value: u64) -> Self {
        let pas = (0..8)
            .map(|i| ((value >> 8 * i) as u8) & 0b111)
            .map(MemoryType::from)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        Self(pas)
    }
}

impl From<PageAttributes> for u64 {
    fn from(value: PageAttributes) -> Self {
        value.0.into_iter().enumerate().fold(0x0, |acc, (i, pa)| {
            acc | ((<MemoryType as Into<u8>>::into(pa) as u64) << (i * 8))
        })
    }
}
