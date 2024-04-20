#![allow(dead_code)]

use alloc::vec::Vec;

use crate::mem::PhysicalAddress;

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
#[repr(C, packed)]
pub(super) struct LocalApic {
    pub pid: u8,
    pub aid: u8,
    pub flags: u32,
}
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub(super) struct IoApic {
    pub ioaid: u8,
    pub reserved: u8,
    pub ioapic_addr: u32,
    pub gsib: u32,
}
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub(super) struct IoApicIntSourceOverride {
    pub bus_source: u8,
    pub irq_source: u8,
    pub gsi: u32,
    pub flags: u16,
}
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub(super) struct IoApicNmiSource {
    nmi_source: u8,
    reserved: u8,
    flags: u16,
    gsi: u32,
}
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub(super) struct LApicNmi {
    pid: u8,
    flags: u16,
    lint: u8,
}
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub(super) struct LApicAddrOverride {
    reserved: u16,
    lapic_physical_addr: u64,
}
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
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

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub(super) struct HpetEntry {
    pub(super) hardware_rev_id: u8,
    bits: u8,
    pub(super) pci_vendor_id: u16,
    // 0 - system memory, 1 - system I/O
    pub(super) address_space_id: u8,
    pub(super) register_bit_width: u8,
    pub(super) register_bit_offset: u8,
    reserved: u8,
    pub(super) address: u64,
    pub(super) hpet_number: u8,
    pub(super) minimum_tick: u16,
    pub(super) page_protection: u8,
}

impl HpetEntry {
    pub(super) fn comparator_count(&self) -> u8 {
        self.bits & 0x1f
    }
    pub(super) fn counter_size(&self) -> bool {
        self.bits & (1 << 5) != 0
    }
    pub(super) fn legacy_replacement(&self) -> bool {
        self.bits & (1 << 7) != 0
    }
}

macro_rules! madt_type {
    ($mt: ident, $addr: ident, $cur_len: ident) => {
        Some(MadtEntry::$mt(unsafe {
            *($addr.byte_add($cur_len) as *const $mt)
        }))
    };
}

#[derive(Debug, Clone)]
pub(super) struct RsdtEntries(Vec<u32>);

impl RsdtEntries {
    fn find(&self, signature: &[u8]) -> Option<AcpiSdt> {
        self.0
            .iter()
            .map(|addr| unsafe {
                PhysicalAddress::new(*addr as u64)
                    .to_virt()
                    .unwrap()
                    .as_ref_static() as *const AcpiSdtHeader
            })
            .find(|addr| {
                let table = unsafe { &**addr };
                &table.signature == signature
            })
            .and_then(AcpiSdt::new)
    }

    pub(super) fn find_madt(&self) -> Option<Vec<MadtEntry>> {
        self.find(b"APIC").map(|a| {
            let AcpiSdtType::Madt { entries, .. } = a.fields else {
                unreachable!()
            };
            entries
        })
    }

    pub(super) fn find_hpet(&self) -> Option<AcpiSdt> {
        self.find(b"HPET")
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub(super) enum AcpiSdtType {
    Rsdt(RsdtEntries),
    Madt {
        lapic: u32,
        flags: u32,
        entries: Vec<MadtEntry>,
    },
    Hpet(HpetEntry),
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
                    .map(|i| unsafe { addr.byte_add(header_length + 4 * i) } as *const u32)
                    .map(|addr| unsafe { addr.read_unaligned() })
                    .collect::<Vec<_>>();
                AcpiSdtType::Rsdt(RsdtEntries(fields))
            }
            b"APIC" => {
                crate::println!("MADT address: {:#x?}", addr);
                let lapic =
                    unsafe { (addr.byte_add(header_length) as *const u32).read_unaligned() };
                let flags =
                    unsafe { (addr.byte_add(header_length + 4) as *const u32).read_unaligned() };
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
                    cur_len += record_len as usize - 2;

                    if let Some(entry) = entry {
                        entries.push(entry);
                    }
                }
                assert_eq!(cur_len, header.length as usize);
                AcpiSdtType::Madt {
                    lapic,
                    flags,
                    entries,
                }
            }
            b"HPET" => {
                let hpet = unsafe { *(addr.byte_add(header_length) as *const HpetEntry) };
                AcpiSdtType::Hpet(hpet)
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

        Some(Self { header, fields })
    }
}

pub(super) fn find_rsdt() -> Option<RsdtEntries> {
    // version 2 is not supported as of yet
    find_rsdp()
        .and_then(|rsdp| {
            let rsdt_addr = unsafe {
                PhysicalAddress::new(rsdp.rsdt_addr as u64)
                    .to_virt()
                    .unwrap()
                    .as_ref_static() as *const AcpiSdtHeader
            };
            AcpiSdt::new(rsdt_addr)
        })
        // .filter(|a| matches!(a.fields, AcpiSdtType::Rsdt(_)));
        .and_then(|a| {
            if let AcpiSdtType::Rsdt(rsdt) = a.fields {
                Some(rsdt)
            } else {
                None
            }
        })
}

fn find_rsdp() -> Option<&'static Rsdp> {
    // TODO: Extended BIOS Data Area (EBDA)
    // the main BIOS area below 1 MB
    let start = unsafe {
        PhysicalAddress::new(0x000E0000)
            .to_virt()
            .unwrap()
            .as_const_ptr()
    };
    let end = unsafe {
        PhysicalAddress::new(0x000FFFFF)
            .to_virt()
            .unwrap()
            .as_const_ptr()
    };
    scan_memory_for_rsdp(start, end)
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
