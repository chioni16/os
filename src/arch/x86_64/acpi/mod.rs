#![allow(dead_code)]

extern crate alloc;
use alloc::vec::Vec;

const RSDP_SIGNATURE: &[u8] = b"RSD PTR "; // notice the space at the end

// only supports V1 for now
// doesn't support the extended version
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct Rsdp {
    signature: [u8; 8],
    checksum: u8,
    oemid: [u8; 6],
    revision: u8,
    rsdt_addr: u32, // 4 byte addr
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub(super) struct AcpiSdtHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LocalApic {
    pid: u8,
    aid: u8,
    flags: u32,
}
#[derive(Debug, Clone, Copy)]
pub(super) struct IoApic {
    ioaid: u8,
    reserved: u8,
    ioapic_addr: u32,
    gsib: u32,
}
#[derive(Debug, Clone, Copy)]
pub(super) struct IoApicIntSourceOverride {
    bus_source: u8,
    irq_source: u8,
    gsi: u32,
    flags: u16,
}
#[derive(Debug, Clone, Copy)]
pub(super) struct IoApicNmiSource {
    nmi_source: u8,
    reserved: u8,
    flags: u16,
    gsi: u32,
}
#[derive(Debug, Clone, Copy)]
pub(super) struct LApicNmi {
    pid: u8,
    flags: u16,
    lint: u8,
}
#[derive(Debug, Clone, Copy)]
pub(super) struct LApicAddrOverride {
    reserved: u16,
    lapic_physical_addr: u64,
}
#[derive(Debug, Clone, Copy)]
pub(super) struct X2Apic {
    reserved: u16,
    id: u32,
    flags: u32,
    acpi_id: u32,
}
#[derive(Debug, Clone, Copy)]
pub(super) enum MadtEntry {
    LocalApic(LocalApic),
    IoApic(IoApic),
    IoApicIntSourceOverride(IoApicIntSourceOverride),
    IoApicNmiSource(IoApicNmiSource),
    LApicNmi(LApicNmi),
    LApicAddrOverride(LApicAddrOverride),
    X2Apic(X2Apic),
}

macro_rules! madt_type {
    ($mt: ident, $addr: ident, $cur_len: ident) => {
        Some(MadtEntry::$mt(unsafe {
            *($addr.byte_add($cur_len) as *const $mt)
        }))
    };
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub(super) enum AcpiSdtType {
    Rsdt(Vec<AcpiSdt>),
    Madt {
        lapic: u32,
        flags: u32,
        entries: Vec<MadtEntry>,
    },
}

#[derive(Debug, Clone)]
pub(super) struct AcpiSdt {
    pub(super) header: &'static AcpiSdtHeader,
    pub(super) fields: AcpiSdtType,
}

impl AcpiSdt {
    fn new(addr: *const AcpiSdtHeader) -> Option<Self> {
        // Assumes that the tables are setup properly
        // The search for tables starts from RSDP and we discover other tables
        // by following the pointers in the previous tables
        let header = unsafe { &*(addr) };
        if calculate_checksum(addr as *const u8, header.length as usize) != 0 {
            return None;
        }
        let header_length = core::mem::size_of::<AcpiSdtHeader>();
        let fields = match &header.signature {
            b"RSDT" => {
                let num_tables = (header.length as usize - header_length) / 4;
                let fields = (0..num_tables)
                    .map(|i| unsafe { *(addr.byte_add(header_length + 4 * i) as *const u32) })
                    .map(|addr| addr as *const AcpiSdtHeader)
                    .map(AcpiSdt::new)
                    .flatten()
                    .collect::<Vec<_>>();
                AcpiSdtType::Rsdt(fields)
            }
            b"APIC" => {
                let lapic = unsafe { *(addr.byte_add(header_length) as *const u32) };
                let flags = unsafe { *(addr.byte_add(header_length + 4) as *const u32) };
                let mut cur_len = header_length + 8;
                let mut entries = Vec::new();
                while cur_len < header.length as usize {
                    let entry_type = unsafe { *(addr.byte_add(cur_len) as *const u8) };
                    let record_len = unsafe { *(addr.byte_add(cur_len + 1) as *const u8) };
                    cur_len += 2;
                    let entry = match entry_type {
                        0 => madt_type!(LocalApic, addr, cur_len),
                        1 => madt_type!(IoApic, addr, cur_len),
                        2 => madt_type!(IoApicIntSourceOverride, addr, cur_len),
                        3 => madt_type!(IoApicNmiSource, addr, cur_len),
                        4 => madt_type!(LApicNmi, addr, cur_len),
                        5 => madt_type!(LApicAddrOverride, addr, cur_len),
                        9 => madt_type!(X2Apic, addr, cur_len),
                        et => {
                            crate::println!("unsupported APIC entry type: {}", et);
                            None
                        }
                    };
                    cur_len += record_len as usize;
                    if let Some(entry) = entry {
                        entries.push(entry);
                    }
                }
                AcpiSdtType::Madt {
                    lapic,
                    flags,
                    entries,
                }
            }
            o => {
                crate::println!(
                    "Unsupported ACPI System Table: {} @ {:x}",
                    core::str::from_utf8(o).unwrap(),
                    addr as usize
                );
                return None;
            }
        };
        let a = Some(Self { header, fields });
        // crate::println!("ACPI end: {:?}", a);
        a
    }
}

pub(super) fn find_rsdt() -> Option<AcpiSdt> {
    // version 2 is not supported as of yet
    find_rsdp()
        .and_then(|rsdp| AcpiSdt::new(rsdp.rsdt_addr as *const AcpiSdtHeader))
        .filter(|a| matches!(a.fields, AcpiSdtType::Rsdt(_)))
}

fn find_rsdp() -> Option<&'static Rsdp> {
    // TODO: Extended BIOS Data Area (EBDA)
    // the main BIOS area below 1 MB
    scan_memory_for_rsdp(0x000E0000 as *const u8, 0x000FFFFF as *const u8)
}

fn scan_memory_for_rsdp(mut start: *const u8, end: *const u8) -> Option<&'static Rsdp> {
    // always occurs at 16 byte boundaries
    start = unsafe { start.byte_add(start.align_offset(16)) };
    while (start as usize) < (end as usize) {
        let candidate = unsafe { core::slice::from_raw_parts(start, RSDP_SIGNATURE.len()) };
        if candidate == RSDP_SIGNATURE
            && calculate_checksum(start, core::mem::size_of::<Rsdp>()) == 0
        {
            return Some(unsafe { &*(start as *const Rsdp) });
        }
        start = unsafe { start.byte_add(16) };
    }
    None
}

fn calculate_checksum(start_addr: *const u8, length: usize) -> u8 {
    (0..length)
        .map(|i| unsafe { *start_addr.byte_add(i) })
        .fold(0, |acc, val| acc.wrapping_add(val))
}
