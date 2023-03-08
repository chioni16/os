use super::port::{io_wait, Port};
use bitflags::bitflags;
use lazy_static::lazy_static;
use paste::paste;
use spin::Mutex;

const MASTER_CMD_PORT: u16 = 0x0020;
const MASTER_DATA_PORT: u16 = 0x0021;
const SLAVE_CMD_PORT: u16 = 0x00a0;
const SLAVE_DATA_PORT: u16 = 0x00a1;

bitflags! {
    struct ICW1: u8 {
        const ICW4	    = 0x01;		/* ICW4 (not) needed */
        const SINGLE	= 0x02;		/* Single (cascade) mode */
        const INTERVAL4	= 0x04;		/* Call address interval 4 (8) */
        const LEVEL	    = 0x08;		/* Level triggered (edge) mode */
        const INIT	    = 0x10;		/* Initialization - required! */
    }

    struct ICW4: u8 {
        const IS_8086    = 0x01;	/* 8086/88 (MCS-80/85) mode */
        const AUTO	     = 0x02;	/* Auto (normal) EOI */
        const BUF_SLAVE	 = 0x08;	/* Buffered mode/slave */
        const BUF_MASTER = 0x0C;	/* Buffered mode/master */
        const SFNM	     = 0x10;	/* Special fully nested (not) */
    }

    struct Mask: u8 {
        const IRQ0 = 0x01;
        const IRQ1 = 0x02;
        const IRQ2 = 0x04;
        const IRQ3 = 0x08;
        const IRQ4 = 0x10;
        const IRQ5 = 0x20;
        const IRQ6 = 0x40;
        const IRQ7 = 0x80;
    }

    pub(super) struct IRQ: u16  {
        const IRQ0 = 0x0001;
        const IRQ1 = 0x0002;
        const IRQ2 = 0x0004;
        const IRQ3 = 0x0008;
        const IRQ4 = 0x0010;
        const IRQ5 = 0x0020;
        const IRQ6 = 0x0040;
        const IRQ7 = 0x0080;

        const IRQ8  = 0x0100;
        const IRQ9  = 0x0200;
        const IRQ10 = 0x0400;
        const IRQ11 = 0x0800;
        const IRQ12 = 0x1000;
        const IRQ13 = 0x2000;
        const IRQ14 = 0x4000;
        const IRQ15 = 0x8000;
    }
}

struct Pic {
    cmd: Port,
    data: Port,
    offset: u8,
}

impl Pic {
    fn new(cmd_port: u16, data_port: u16, offset: u8) -> Self {
        Self {
            cmd: Port::new(cmd_port),
            data: Port::new(data_port),
            offset,
        }
    }

    pub(super) unsafe fn remap(&mut self, other: u8) {
        // save mask
        let mask: u8 = self.data.read::<u8>();
        // ICW1 start initialisation sequence
        self.cmd.write((ICW1::INIT | ICW1::ICW4).bits());
        io_wait();
        // ICW2 set vector offset
        self.data.write(self.offset & !0x7);
        io_wait();
        // ICW3 set address of the other PIC
        self.data.write(other);
        io_wait();
        // ICW4
        self.data.write(ICW4::IS_8086.bits());
        io_wait();
        // restore mask
        self.data.write(mask);
    }

    unsafe fn send_eoi(&mut self) {
        self.cmd.write::<u8>(0x20);
    }

    unsafe fn disable(&mut self) {
        // mask all the interrupt lines
        self.data.write::<u8>(0xff);
    }

    unsafe fn get_mask(&mut self) -> u8 {
        self.data.read()
    }

    unsafe fn set_mask(&mut self, irq: u8) {
        assert!(irq < 8);
        let irq = Mask::from_bits(1 << irq).unwrap();
        let mask = Mask::from_bits(self.data.read()).unwrap();
        self.data.write((mask | irq).bits());
    }

    unsafe fn clear_mask(&mut self, irq: u8) {
        assert!(irq < 8);
        let irq = Mask::from_bits(1 << irq).unwrap();
        let mask = Mask::from_bits(self.data.read()).unwrap();
        let data = mask - irq;
        crate::println!("clear: {:?}", data);
        self.data.write((data).bits());
    }

    unsafe fn get_irr(&mut self) -> u8 {
        self.cmd.write::<u8>(0x0a);
        self.cmd.read()
    }

    unsafe fn get_isr(&mut self) -> u8 {
        self.cmd.write::<u8>(0x0b);
        self.cmd.read()
    }
}

lazy_static! {
    static ref MASTER_PIC: Mutex<Pic> =
        Mutex::new(Pic::new(MASTER_CMD_PORT, MASTER_DATA_PORT, 0x20));
    static ref SLAVE_PIC: Mutex<Pic> = Mutex::new(Pic::new(SLAVE_CMD_PORT, SLAVE_DATA_PORT, 0x28));
}

pub(super) unsafe fn remap() {
    MASTER_PIC.lock().remap(4);
    SLAVE_PIC.lock().remap(2);
}

pub(super) unsafe fn disable() {
    SLAVE_PIC.lock().disable();
    MASTER_PIC.lock().disable();
}

pub(super) unsafe fn send_eoi(irq: u8) {
    assert!(irq < 16);
    if irq >= 8 {
        SLAVE_PIC.lock().send_eoi();
    }
    MASTER_PIC.lock().send_eoi();
}

pub(super) unsafe fn set_mask(irq: u8) {
    assert!(irq < 16);
    if irq < 8 {
        MASTER_PIC.lock().set_mask(irq);
    } else {
        SLAVE_PIC.lock().set_mask(irq - 8);
    }
}

pub(super) unsafe fn clear_mask(irq: u8) {
    assert!(irq < 16);
    if irq < 8 {
        MASTER_PIC.lock().clear_mask(irq);
    } else {
        SLAVE_PIC.lock().clear_mask(irq - 8);
    }
}

macro_rules! get {
    ($name: ident) => {
        paste! {
            pub(super) unsafe fn [<get_$name>]() -> IRQ {
                let master = MASTER_PIC.lock().[<get_$name>]() as u16;
                let slave = SLAVE_PIC.lock().[<get_$name>]() as u16;
                // crate::println!("get:\n{:b}\n{:b}", Mask::from_bits(master as u8).unwrap(), Mask::from_bits(slave as u8).unwrap());
                IRQ::from_bits((slave << 8) | master).unwrap()
            }
        }
    };
}

get!(mask);
get!(irr);
get!(isr);

// pub(super) unsafe fn get_mask() -> IRQ {
//     let master = MASTER_PIC.lock().get_mask() as u16;
//     let slave = SLAVE_PIC.lock().get_mask() as u16;
//     IRQ::from_bits((master << 8) | slave).unwrap()
// }

// pub(super) unsafe fn get_irr() -> IRQ {
//     let master = MASTER_PIC.lock().get_irr() as u16;
//     let slave = SLAVE_PIC.lock().get_irr() as u16;
//     IRQ::from_bits((master << 8) | slave).unwrap()
// }

// pub(super) unsafe fn get_isr() -> IRQ {
//     let master = MASTER_PIC.lock().get_isr() as u16;
//     let slave = SLAVE_PIC.lock().get_isr() as u16;
//     IRQ::from_bits((master << 8) | slave).unwrap()
// }

pub(super) fn init() {
    unsafe {
        remap();
        disable();
        clear_mask(1);
        clear_mask(2);
        clear_mask(11);
        crate::println!("{:?}", get_mask());
    }
}