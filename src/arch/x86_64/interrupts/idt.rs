use super::isr::HandlerFn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
struct InterruptDescriptor {
    offset_1: u16,
    selector: u16,
    options: u16,
    offset_2: u16,
    offset_3: u32,
    reserved: u32,
}

impl InterruptDescriptor {
    fn new(handler: HandlerFn, options: Options) -> Self {
        let handler = handler as u64;
        Self {
            offset_1: handler as u16,
            selector: get_cs(),
            options: options.into(),
            offset_2: (handler >> 16) as u16,
            offset_3: (handler >> 32) as u32,
            reserved: 0,
        }
    }

    fn missing() -> Self {
        Self {
            offset_1: 0,
            selector: get_cs(),
            options: Options::minimal().into(),
            offset_2: 0,
            offset_3: 0,
            reserved: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Options {
    stack_table: u8,
    gate_type: GateType,
    dpl: u8,
    present: bool,
}

impl Options {
    fn minimal() -> Self {
        Self {
            stack_table: 0,
            gate_type: GateType::Interrupt,
            dpl: 0,
            present: false,
        }
    }
}

impl From<u16> for Options {
    fn from(value: u16) -> Self {
        Self {
            stack_table: (value & 0b111) as u8, // 0-2
            gate_type: GateType::try_from(((value >> 8) & 1) as u8).unwrap(), // 8
            dpl: ((value >> 13) & 0b11) as u8,  // 13-14
            present: (value >> 15) > 0,         // 15
        }
    }
}

impl From<Options> for u16 {
    fn from(value: Options) -> Self {
        (value.present as u16) << 15
            | (value.dpl as u16) << 13
            | 0 << 12
            | (0b111 as u16) << 9
            | ((value.gate_type as u8) as u16) << 8
            | 0 << 3
            | value.stack_table as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum GateType {
    // If this bit is 0, interrupts are disabled when this handler is called.
    Interrupt = 0,
    Trap = 1,
}

impl TryFrom<u8> for GateType {
    type Error = u8;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Interrupt),
            1 => Ok(Self::Trap),
            _ => Err(value),
        }
    }
}

fn get_cs() -> u16 {
    let cs: u16;
    unsafe {
        core::arch::asm!("mov {:x}, cs", out(reg) cs);
    }
    cs
}

fn lidt(dtp: &DescriptorTablePointer) {
    unsafe {
        core::arch::asm!("lidt [{}]", in(reg) dtp, options(readonly, nostack, preserves_flags));
    }
}

// #[repr(transparent)]
#[repr(C, align(16))]
pub(super) struct InterruptDescriptorTable([InterruptDescriptor; 48]);

impl InterruptDescriptorTable {
    pub(super) fn new() -> Self {
        Self([InterruptDescriptor::missing(); 48])
    }

    pub(super) fn add_handler(&mut self, index: usize, handler: HandlerFn) {
        let options = Options {
            stack_table: 0,
            gate_type: GateType::Interrupt,
            dpl: 0,
            present: true,
        };
        self.0[index] = InterruptDescriptor::new(handler, options);
    }

    pub(super) fn load(&'static self) {
        let dtp = DescriptorTablePointer {
            size: (core::mem::size_of_val(self) - 1) as u16,
            offset: self as *const Self as u64,
        };

        lidt(&dtp);
    }
}

#[repr(C, packed)]
struct DescriptorTablePointer {
    size: u16,
    offset: u64,
}
