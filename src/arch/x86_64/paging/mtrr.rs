extern crate alloc;
use alloc::vec::Vec;

use crate::{arch::x86_64::{rdmsr, wrmsr}, mem::PhysicalAddress};
use core::arch::x86_64::{CpuidResult, __cpuid};

pub(super) fn supports_mtrr() -> bool {
    let CpuidResult { edx, .. } = unsafe { __cpuid(1) };
    edx & (1 << 12) != 0
}

// SAFETY: only run if the processor supports MTRR (use cpuid to get this)
// NOTE: read only, writing causes #GP
pub(super) unsafe fn read_mtrr_cap_msr() -> MtrrCapability {
    let msr = unsafe { rdmsr(0xfe) };
    msr.into()
}

// SAFETY: only run if the processor supports MTRR (use cpuid to get this)
pub(super) unsafe fn read_mtrr_default_type_reg() -> MtrrDefaultType {
    let msr = unsafe { rdmsr(0x2ff) };
    msr.into()
}

// SAFETY: only run if the processor supports MTRR (use cpuid to get this)
// Setting reserved memory type will cause #GP (maybe not immediately but when the processor tries to use it)
// need to be consistent across all the processors
pub(super) unsafe fn write_mtrr_default_type_reg(def_type: MtrrDefaultType) {
    wrmsr(0x2ff, def_type.into());
    // no need to flush TLBs as unlike PAT, MTRRs are associated with the physical memory and not virtual memory
}

// SAFETY: only run if the processor supports MTRR (use cpuid to get this)
// and fixed_range_regs are supported and enabled
pub(super) unsafe fn read_fixed_range_mtrr(addr: PhysicalAddress) -> Option<MemoryType> {
    get_fixed_range_reg_msr(addr).map(|(msr, bit_offset)| {
        let msr_val = unsafe {rdmsr(msr)};
        let mt = (msr_val >> bit_offset) as u8;
        mt.into()
    })
}

// SAFETY: only run if the processor supports MTRR (use cpuid to get this)
// and fixed_range_regs are supported and enabled
// Setting reserved memory type will cause #GP (maybe not immediately but when the processor tries to use it)
// need to be consistent across all the processors
pub(super) unsafe fn write_fixed_range_mtrr(addr: PhysicalAddress, mt: MemoryType) {
    let Some((msr, bit_offset)) = get_fixed_range_reg_msr(addr) else {
        panic!("address not covered by fixed range regs");
    };
    let msr_val = unsafe {rdmsr(msr)};

    let mt: u8 = mt.into();
    let mt = (mt as u64) << bit_offset;

    let value = (msr_val & (!0xff << bit_offset)) | mt;
    unsafe {wrmsr(msr, value)};
}

// returns the msr and the starting bit within the msr corresponding to the physical address
fn get_fixed_range_reg_msr(addr: PhysicalAddress) -> Option<(u32, u8)> {
    let addr = addr.to_inner();
    let (msr, unit) = match addr {
        0x00000..=0x7ffff => (0x250, (addr - 0x00000) / ((512 * 1024) / 8)),
        0x80000..=0x9ffff => (0x258, (addr - 0x80000) / ((128 * 1024) / 8)),
        0xa0000..=0xbffff => (0x259, (addr - 0xa0000) / ((128 * 1024) / 8)),
        0xc0000..=0xc7fff => (0x268, (addr - 0xc0000) / ((32 * 1024) / 8)),
        0xc8000..=0xcffff => (0x269, (addr - 0xc8000) / ((32 * 1024) / 8)),
        0xd0000..=0xd7fff => (0x26a, (addr - 0xd0000) / ((32 * 1024) / 8)),
        0xd8000..=0xdffff => (0x26b, (addr - 0xd8000) / ((32 * 1024) / 8)),
        0xe0000..=0xe7fff => (0x26c, (addr - 0xe0000) / ((32 * 1024) / 8)),
        0xe8000..=0xeffff => (0x26d, (addr - 0xe8000) / ((32 * 1024) / 8)),
        0xf0000..=0xf7fff => (0x26e, (addr - 0xf0000) / ((32 * 1024) / 8)),
        0xf8000..=0xfffff => (0x26f, (addr - 0xf8000) / ((32 * 1024) / 8)),
        // Supports only the first 1 MiB of physical address space
        _ => return None,
    };

    assert!(unit < 8);

    Some((msr, unit as u8 * 8))
}

#[derive(Debug)]
pub(super) struct MtrrCapability {
    pub(super) variable_regs_count: u8,
    pub(super) supports_fixed_range_regs: bool,
    pub(super) supports_write_combining: bool,
    pub(super) supports_smrr_interface: bool,
}

impl From<u64> for MtrrCapability {
    fn from(value: u64) -> Self {
        Self {
            variable_regs_count: value as u8,
            supports_fixed_range_regs: value & (1 << 8) != 0,
            supports_write_combining: value & (1 << 10) != 0,
            supports_smrr_interface: value & (1 << 11) != 0,
        }
    }
}

#[derive(Debug)]
pub(super) struct MtrrDefaultType {
    pub(super) default_type: MemoryType,
    // when enabled, fixed range regs take priority over the variable range regs for the overlapping regions
    pub(super) fixed_range_mtrr_enabled: bool,
    // when disabled, default_type is used for all the memory regions
    pub(super) mtrr_enabled: bool,
}

impl From<u64> for MtrrDefaultType {
    fn from(value: u64) -> Self {
        Self {
            default_type: (value as u8).into(),
            fixed_range_mtrr_enabled: value & (1 << 10) != 0,
            mtrr_enabled: value & (1 << 11) != 0,
        }
    }
}

impl From<MtrrDefaultType> for u64 {
    fn from(value: MtrrDefaultType) -> Self {
        let mut msr = <MemoryType as Into<u8>>::into(value.default_type) as u64;
        msr |= (value.fixed_range_mtrr_enabled as u64) << 10;
        msr |= (value.mtrr_enabled as u64) << 11;
        msr
    }
}

// different from PAT MemoryType in that it lacks UC- type
#[derive(Debug, Clone, Copy)]
pub(super) enum MemoryType {
    Uncacheable,
    WriteCombining,
    WriteThrough,
    WriteProtected,
    WriteBack,
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

#[derive(Debug)]
pub(super) struct MemoryTypes(pub(super) [MemoryType; 8]);

impl From<u64> for MemoryTypes {
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

impl From<MemoryTypes> for u64 {
    fn from(value: MemoryTypes) -> Self {
        value.0.into_iter().enumerate().fold(0x0, |acc, (i, pa)| {
            acc | ((<MemoryType as Into<u8>>::into(pa) as u64) << (i * 8))
        })
    }
}
