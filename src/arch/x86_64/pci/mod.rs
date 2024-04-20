use alloc::{boxed::Box, collections::VecDeque};
use core::mem::transmute;
use lazy_static::lazy_static;
use paste::paste;

use super::port::Port;
use spin::Mutex;

lazy_static! {
    static ref CONFIG_ADDR: Mutex<Port> = Mutex::new(Port::new(0xCF8));
    static ref CONFIG_DATA: Mutex<Port> = Mutex::new(Port::new(0xCFC)); // CONFIG_ADDR + 4
}

// Register	Offset	Bits 31-24	Bits 23-16	Bits 15-8	 Bits 7-0
// 0x0	     0x0	Device ID	            Vendor ID
// 0x1	     0x4	Status	                Command
// 0x2	     0x8	ClassCode	Subclass	Prog IF	     Revision ID
// 0x3	     0xC	BIST	    HeaderType	LatencyTimer Cache Line Size
#[derive(Debug, Clone, Copy)]
pub(super) struct PciDevice {
    pub(super) bus: u8,
    pub(super) device: u8,
    pub(super) function: u8,
    pub(super) device_id: u16,
    pub(super) vendor_id: u16,

    pub(super) header_type: HeaderType,
}

macro_rules! write_byte {
    ($name: ident, $offset: expr, $index: expr) => {
        paste! {
            pub(super) fn [<write_$name>](&mut self, new: u8) {
                let mut cur = self.read_at_offset($offset).to_le_bytes();
                cur[$index] = new;
                self.write_at_offset($offset, u32::from_le_bytes(cur));
            }
        }
    };
}
macro_rules! write_word {
    ($name: ident, $offset: expr, $index: expr) => {
        paste! {
            pub(super) fn [<write_$name>](&mut self, new: u16) {
                let mut cur = unsafe { transmute::<u32, [u16; 2]>(self.read_at_offset($offset)) };
                cur[$index] = new;
                self.write_at_offset($offset, unsafe { transmute::<[u16; 2], u32>(cur) });
            }
        }
    };
}

impl PciDevice {
    fn read_at_offset(&mut self, offset: u8) -> u32 {
        read(self.bus, self.device, self.function, offset)
    }
    pub(super) fn status(&mut self) -> u16 {
        unsafe { transmute::<u32, [u16; 2]>(self.read_at_offset(0x4))[1] }
    }
    pub(super) fn command(&mut self) -> u16 {
        unsafe { transmute::<u32, [u16; 2]>(self.read_at_offset(0x4))[0] }
    }
    pub(super) fn class_code(&mut self) -> u8 {
        self.read_at_offset(0x8).to_le_bytes()[3]
    }
    pub(super) fn subclass(&mut self) -> u8 {
        self.read_at_offset(0x8).to_le_bytes()[2]
    }
    pub(super) fn prog_if(&mut self) -> u8 {
        self.read_at_offset(0x8).to_le_bytes()[1]
    }
    pub(super) fn revision_id(&mut self) -> u8 {
        self.read_at_offset(0x8).to_le_bytes()[0]
    }
    pub(super) fn bist(&mut self) -> u8 {
        self.read_at_offset(0xc).to_le_bytes()[3]
    }
    pub(super) fn header_type(&mut self) -> u8 {
        self.read_at_offset(0xc).to_le_bytes()[2] & !(1 << 7)
    }
    pub(super) fn latency_timer(&mut self) -> u8 {
        self.read_at_offset(0xc).to_le_bytes()[1]
    }
    pub(super) fn cache_line_size(&mut self) -> u8 {
        self.read_at_offset(0xc).to_le_bytes()[0]
    }

    fn write_at_offset(&mut self, offset: u8, value: u32) {
        write(self.bus, self.device, self.function, offset, value);
    }

    // write_word!(status, 0x4, 1);
    write_word!(command, 0x4, 0);
    write_byte!(class_code, 0x8, 3);
    write_byte!(subclass, 0x8, 2);
    write_byte!(prog_if, 0x8, 1);
    write_byte!(revision_id, 0x8, 0);
    write_byte!(bist, 0xc, 3);
    // write_byte!(header_type, 0xc, 2);
    write_byte!(latency_timer, 0xc, 1);
    write_byte!(cache_line_size, 0xc, 0);
}

#[derive(Debug, Clone, Copy)]
pub(super) enum HeaderType {
    Type0(Type0),
    // unimplemented
    // not interesting for what I've in mind at the moment
    Type1,
    Type2,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Type0 {}

impl Type0 {
    pub(super) fn bars(&mut self, bus: u8, device: u8, function: u8) -> [BaseAddrReg; 6] {
        [
            BaseAddrReg::new(bus, device, function, 0x10 + 4 * 0),
            BaseAddrReg::new(bus, device, function, 0x10 + 4 * 1),
            BaseAddrReg::new(bus, device, function, 0x10 + 4 * 2),
            BaseAddrReg::new(bus, device, function, 0x10 + 4 * 3),
            BaseAddrReg::new(bus, device, function, 0x10 + 4 * 4),
            BaseAddrReg::new(bus, device, function, 0x10 + 4 * 5),
        ]
    }
    pub(super) fn cis_pointer(&mut self, bus: u8, device: u8, function: u8) -> u32 {
        read(bus, device, function, 0x28)
    }
    pub(super) fn ssid(&mut self, bus: u8, device: u8, function: u8) -> u16 {
        unsafe { transmute::<u32, [u16; 2]>(read(bus, device, function, 0x2c))[1] }
    }
    pub(super) fn svid(&mut self, bus: u8, device: u8, function: u8) -> u16 {
        unsafe { transmute::<u32, [u16; 2]>(read(bus, device, function, 0x2c))[0] }
    }
    pub(super) fn erba(&mut self, bus: u8, device: u8, function: u8) -> u32 {
        read(bus, device, function, 0x30)
    }
    pub(super) fn capabilities(&mut self, bus: u8, device: u8, function: u8) -> u8 {
        read(bus, device, function, 0x34).to_le_bytes()[0];
        todo!()
    }
    pub(super) fn max_latency(&mut self, bus: u8, device: u8, function: u8) -> u8 {
        read(bus, device, function, 0x3c).to_le_bytes()[3]
    }
    pub(super) fn min_grant(&mut self, bus: u8, device: u8, function: u8) -> u8 {
        read(bus, device, function, 0x3c).to_le_bytes()[2]
    }
    pub(super) fn interrupt_pin(&mut self, bus: u8, device: u8, function: u8) -> u8 {
        read(bus, device, function, 0x3c).to_le_bytes()[1]
    }
    pub(super) fn interrupt_line(&mut self, bus: u8, device: u8, function: u8) -> u8 {
        read(bus, device, function, 0x3c).to_le_bytes()[0]
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum BaseAddrRegType {
    Io(u32),
    // not completely implemented yet
    Mem,
}

impl From<u32> for BaseAddrRegType {
    fn from(value: u32) -> Self {
        if value & 0x1 == 0 {
            Self::Mem
        } else {
            Self::Io(value & !0b11)
        }
    }
}

impl Into<u32> for BaseAddrRegType {
    fn into(self) -> u32 {
        match self {
            Self::Io(port) => port | 0b1,
            Self::Mem => unimplemented!(),
        }
    }
}

#[derive(Debug)]
pub(super) struct BaseAddrReg {
    bus: u8,
    device: u8,
    function: u8,
    offset: u8,
}

impl BaseAddrReg {
    fn new(bus: u8, device: u8, function: u8, offset: u8) -> Self {
        Self {
            bus,
            device,
            function,
            offset,
        }
    }

    pub(super) fn read(&mut self) -> BaseAddrRegType {
        read(self.bus, self.device, self.function, self.offset).into()
    }

    pub(super) fn write(&mut self, new: BaseAddrRegType) {
        write(
            self.bus,
            self.device,
            self.function,
            self.offset,
            new.into(),
        )
    }
}
pub(super) struct PciEnumerate {
    // geographical addresses
    cur_bus: u8,      // 256 buses   - 8 bits
    cur_device: u8,   // 32 devices  - 5 bits
    cur_function: u8, // 8 functions - 3 bits
    next_buses: VecDeque<u8>,
}

impl PciEnumerate {
    pub(super) fn new() -> Self {
        Self {
            cur_bus: 0,
            cur_device: 0,
            cur_function: 0,
            next_buses: VecDeque::new(),
        }
    }

    fn incr(&mut self) -> bool {
        self.incr_function() && self.incr_device() && self.incr_bus()
    }

    fn incr_function(&mut self) -> bool {
        let mut max_functions = 8;
        if self.cur_function == 0 && !has_multiple_functions(self.cur_bus, self.cur_device) {
            max_functions = 1;
        }

        self.cur_function += 1;
        if self.cur_function >= max_functions {
            self.cur_function = 0;
            true
        } else {
            false
        }
    }
    fn incr_device(&mut self) -> bool {
        self.cur_device += 1;
        if self.cur_device >= 32 {
            self.cur_device = 0;
            true
        } else {
            false
        }
    }
    fn incr_bus(&mut self) -> bool {
        self.next_buses.pop_front().map_or(true, |b| {
            self.cur_bus = b;
            false
        })
    }
}

impl Iterator for PciEnumerate {
    type Item = PciDevice;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.incr() {
                return None;
            }

            let [vendor_id, device_id]: [u16; 2] =
                unsafe { transmute(read(self.cur_bus, self.cur_device, self.cur_function, 0)) };
            if device_id == 0x0000
                || device_id == 0xffff
                || vendor_id == 0x0000
                || vendor_id == 0xffff
            {
                continue;
            }
            let header_type = read(self.cur_bus, self.cur_device, self.cur_function, 12)
                .to_le_bytes()[2]
                & !(1 << 7);
            let header_type = match header_type {
                0x0 => HeaderType::Type0(Type0 {}),
                0x1 => HeaderType::Type1,
                0x2 => HeaderType::Type2,
                _ => unreachable!(),
            };
            return Some(PciDevice {
                bus: self.cur_bus,
                device: self.cur_device,
                function: self.cur_function,
                device_id,
                vendor_id,
                header_type,
            });
        }
    }
}

fn get_geo_addr(bus: u8, device: u8, function: u8, reg_offset: u8) -> u32 {
    let a = (0x1u32 << 31)
        | ((bus as u32) << 16)
        | ((device as u32 & 0x1f) << 11)
        | ((function as u32 & 0x07) << 8)
        | (reg_offset as u32 & 0xfc); // align: 4 byte
    a
}

fn read(bus: u8, device: u8, function: u8, reg_offset: u8) -> u32 {
    let geo_addr = get_geo_addr(bus, device, function, reg_offset);
    let data: u32 = unsafe {
        CONFIG_ADDR.lock().write(geo_addr);
        CONFIG_DATA.lock().read()
    };
    data >> (8 * (reg_offset % 4))
}

fn has_multiple_functions(bus: u8, device: u8) -> bool {
    let header_type = read(bus, device, 0, 0xe);
    header_type & (1 << 7) != 0
}

fn write(bus: u8, device: u8, function: u8, reg_offset: u8, data: u32) {
    let geo_addr = get_geo_addr(bus, device, function, reg_offset);
    unsafe {
        CONFIG_ADDR.lock().write(geo_addr);
        CONFIG_DATA.lock().write(data);
    }
}

pub(super) fn init() {
    for mut p in PciEnumerate::new().filter(|p| p.device_id == 0x8139 && p.vendor_id == 0x10ec) {
        //  data master
        let cur = p.command();
        p.write_command(cur | 0b100);
        crate::println!("{:016b}", p.command());

        if let HeaderType::Type0(mut t) = p.header_type {
            t.bars(p.bus, p.device, p.function)
                .into_iter()
                .for_each(|mut b| {
                    if let BaseAddrRegType::Io(orig) = b.read() {
                        crate::println!("{:x?}", orig);
                        let orig = orig as u16;
                        unsafe {
                            Port::new(orig + 0x52).write::<u8>(0x00);
                            crate::println!("successfully turned it on");

                            let mut cmd = Port::new(orig + 0x37);
                            cmd.write::<u8>(0x10);
                            crate::println!("wrote 0x10 to cmd reg");
                            while cmd.read::<u8>() & 0x10 != 0 {}
                            crate::println!("passed while test");

                            let mut cmd = Port::new(orig + 0x37);
                            cmd.write(0x0cu8);
                            crate::println!("Rx and Tx enabled: {:#b}", cmd.read::<u8>());

                            // let rx_buffer = Box::new([0u8; 8192+16+1500]);
                            let rx_buffer = Box::new([0x0u8; 8192 + 16]);
                            let rx_buffer = Box::into_raw(rx_buffer);
                            crate::println!("rx_buffer: {:#x?}", rx_buffer);
                            let mut rx_buffer_port = Port::new(orig + 0x30);
                            rx_buffer_port.write(rx_buffer as u32);
                            crate::println!(
                                "rx buffer registered @ {:#x}",
                                rx_buffer_port.read::<u32>()
                            );

                            let mut imr = Port::new(orig + 0x3c);
                            imr.write(0x0005u16);
                            crate::println!("IMR set to {:#b}", imr.read::<u16>());

                            let mut rcr_port = Port::new(orig + 0x44);
                            // rcr_port.write::<u32>(0xf | (1 << 7));
                            rcr_port.write::<u32>(0xf);
                            crate::println!("RCR configured to {:#x}", rcr_port.read::<u32>());

                            // let mut capr = Port::new(orig + 0x38);
                            // capr.write::<u16>(0);
                            // crate::println!("CAPR configured to {:#x}", capr.read::<u32>());

                            crate::println!("rtl8159 command: {:b}", cmd.read::<u8>());

                            let base_addr = orig;
                            // 0030h-0033h R/W RBSTART Receive (Rx) Buffer Start Address
                            crate::println!(
                                "RBSTART: {:#x}",
                                Port::new(base_addr + 0x30).read::<u32>()
                            );
                            // 0037h R/W CR Command Register
                            crate::println!(
                                "COMMAND: {:#b}",
                                Port::new(base_addr + 0x37).read::<u8>()
                            );
                            // 0038h-0039h R/W CAPR Current Address of Packet Read
                            crate::println!(
                                "CAPR: {:#x}",
                                Port::new(base_addr + 0x38).read::<u16>()
                            );
                            // 003Ah-003Bh R CBR Current Buffer Address:
                            crate::println!(
                                "CBA: {:#x}",
                                Port::new(base_addr + 0x3a).read::<u16>()
                            );
                            // 003Ch-003Dh R/W IMR Interrupt Mask Register
                            crate::println!(
                                "IMR: {:#b}",
                                Port::new(base_addr + 0x3c).read::<u16>()
                            );
                            // 003Eh-003Fh R/W ISR Interrupt Status Register
                            crate::println!(
                                "ISR: {:#b}",
                                Port::new(base_addr + 0x3e).read::<u16>()
                            );
                            // 0044h-0047h R/W RCR Receive (Rx) Configuration Register
                            crate::println!(
                                "RCR: {:#x}",
                                Port::new(base_addr + 0x44).read::<u32>()
                            );
                        }

                        // b.write(BaseAddrRegType::Io(u32::MAX));
                        // crate::println!("{:x?}", b.read());
                        // b.write(orig);
                        // crate::println!("{:x?}", b.read());
                    }
                });
        }
        crate::println!("pci command: {:b}", p.command());
    }
}
