extern crate alloc;
use alloc::boxed::Box;

use crate::{
    __eh_frame_end, __eh_frame_hdr_end, __eh_frame_hdr_start, __eh_frame_start, mem::VirtualAddress,
};
use core::{ptr::addr_of, slice};
use gimli::{
    BaseAddresses, CfaRule, EhFrame, EhFrameHdr, EhHdrTable, EndianSlice, LittleEndian,
    ParsedEhFrameHdr, Pointer, Register, RegisterRule, UnwindContext, UnwindSection, X86_64,
};

#[derive(Debug)]
pub struct CallFrame {
    pub pc: VirtualAddress,
    // pub symbol: Option<&'static str>,
    // pub sym_off: Option<usize>,
    // pub file_line: Option<(&'static str, u32)>,
}

pub struct EhInfo {
    /// A set of base addresses used for relative addressing.
    base_addrs: BaseAddresses,

    /// The parsed `.eh_frame_hdr` section.
    hdr: &'static ParsedEhFrameHdr<EndianSlice<'static, LittleEndian>>,

    /// The lookup table within the parsed `.eh_frame_hdr` section.
    hdr_table: EhHdrTable<'static, EndianSlice<'static, LittleEndian>>,

    /// The parsed `.eh_frame` containing the call frame information.
    eh_frame: EhFrame<EndianSlice<'static, LittleEndian>>,
}

impl EhInfo {
    pub unsafe fn new() -> Self {
        let mut base_addrs = BaseAddresses::default();
        // We set the `.eh_frame_hdr`’s address in the set of base addresses,
        // this will typically be used to compute the `.eh_frame` pointer.
        base_addrs = base_addrs.set_eh_frame_hdr(addr_of!(__eh_frame_hdr_start) as u64);

        // The `.eh_frame_hdr` is parsed by Gimli. We leak a box for
        // convenience, this gives us a reference with 'static lifetime.
        let hdr = Box::leak(Box::new(
            EhFrameHdr::new(
                unsafe {
                    slice::from_raw_parts(
                        addr_of!(__eh_frame_hdr_start),
                        addr_of!(__eh_frame_hdr_end) as usize
                            - addr_of!(__eh_frame_hdr_start) as usize,
                    )
                },
                LittleEndian,
            )
            .parse(&base_addrs, 8)
            .unwrap(),
        ));

        // We deduce the `.eh_frame` address, only direct pointers are implemented.
        let eh_frame = match hdr.eh_frame_ptr() {
            Pointer::Direct(addr) => addr as *mut u8,
            _ => unimplemented!(),
        };

        // We then add the `.eh_frame` address for addresses relative to that
        // section.
        base_addrs = base_addrs.set_eh_frame(eh_frame as u64);

        // The `.eh_frame` section is then parsed.
        let eh_frame = EhFrame::new(
            unsafe {
                slice::from_raw_parts(
                    eh_frame,
                    addr_of!(__eh_frame_end) as usize - addr_of!(__eh_frame_start) as usize,
                )
            },
            LittleEndian,
        );

        Self {
            base_addrs,
            hdr,
            hdr_table: hdr.table().unwrap(),
            eh_frame,
        }
    }
}

pub struct Unwinder {
    /// The call frame information.
    eh_info: EhInfo,

    /// A `UnwindContext` needed by Gimli for optimizations.
    unwind_ctx: UnwindContext<EndianSlice<'static, LittleEndian>>,

    /// The current values of registers. These values are updated as we restore
    /// register values.
    regs: RegisterSet,

    /// The current CFA address.
    cfa: u64,

    /// Is it the first iteration?
    is_first: bool,
}

impl Unwinder {
    pub fn new(eh_info: EhInfo, register_set: RegisterSet) -> Self {
        crate::println!("unwinder new 1");
        let unwind_ctx = UnwindContext::new();
        crate::println!("unwinder new 2");
        Self {
            eh_info,
            unwind_ctx,
            regs: register_set,
            cfa: 0,
            is_first: true,
        }
    }

    pub fn next(&mut self) -> Result<Option<CallFrame>, UnwinderError> {
        let pc = self.regs.get_pc().ok_or(UnwinderError::NoPcRegister)?;

        if self.is_first {
            self.is_first = false;
            return Ok(Some(CallFrame {
                pc: VirtualAddress::new(pc),
            }));
        }

        let row = self
            .eh_info
            .hdr_table
            .unwind_info_for_address(
                &self.eh_info.eh_frame,
                &self.eh_info.base_addrs,
                &mut self.unwind_ctx,
                pc,
                |section, bases, offset| section.cie_from_offset(bases, offset),
            )
            .map_err(|_| UnwinderError::NoUnwindInfo)?;

        match row.cfa() {
            CfaRule::RegisterAndOffset { register, offset } => {
                let reg_val = self
                    .regs
                    .get(*register)
                    .ok_or(UnwinderError::CfaRuleUnknownRegister(*register))?;
                self.cfa = (reg_val as i64 + offset) as u64;
            }
            _ => return Err(UnwinderError::UnsupportedCfaRule),
        }

        for reg in RegisterSet::iter() {
            match row.register(reg) {
                RegisterRule::Undefined => self.regs.undef(reg),
                RegisterRule::SameValue => (),
                RegisterRule::Offset(offset) => {
                    let ptr = (self.cfa as i64 + offset) as u64 as *const usize;
                    self.regs.set(reg, unsafe { ptr.read() } as u64)?;
                }
                _ => return Err(UnwinderError::UnimplementedRegisterRule),
            }
        }

        let pc = self.regs.get_ret().ok_or(UnwinderError::NoReturnAddr)? - 1;
        self.regs.set_pc(pc);
        self.regs.set_stack_ptr(self.cfa);

        Ok(Some(CallFrame {
            pc: VirtualAddress::new(pc),
        }))
    }
}

#[derive(Debug)]
pub enum UnwinderError {
    UnexpectedRegister(Register),
    UnsupportedCfaRule,
    UnimplementedRegisterRule,
    CfaRuleUnknownRegister(Register),
    NoUnwindInfo,
    NoPcRegister,
    NoReturnAddr,
}

#[derive(Debug, Default)]
pub struct RegisterSet {
    pub rip: Option<u64>,
    pub rsp: Option<u64>,
    pub rbp: Option<u64>,
    pub ret: Option<u64>,
}

impl RegisterSet {
    fn get(&self, reg: Register) -> Option<u64> {
        match reg {
            X86_64::RSP => self.rsp,
            X86_64::RBP => self.rbp,
            X86_64::RA => self.ret,
            _ => None,
        }
    }

    fn set(&mut self, reg: Register, val: u64) -> Result<(), UnwinderError> {
        *match reg {
            X86_64::RSP => &mut self.rsp,
            X86_64::RBP => &mut self.rbp,
            X86_64::RA => &mut self.ret,
            _ => return Err(UnwinderError::UnexpectedRegister(reg)),
        } = Some(val);

        Ok(())
    }

    fn undef(&mut self, reg: Register) {
        *match reg {
            X86_64::RSP => &mut self.rsp,
            X86_64::RBP => &mut self.rbp,
            X86_64::RA => &mut self.ret,
            _ => return,
        } = None;
    }

    fn get_pc(&self) -> Option<u64> {
        self.rip
    }

    fn set_pc(&mut self, val: u64) {
        self.rip = Some(val);
    }

    fn get_ret(&self) -> Option<u64> {
        self.ret
    }

    fn set_stack_ptr(&mut self, val: u64) {
        self.rsp = Some(val);
    }

    fn iter() -> impl Iterator<Item = Register> {
        [X86_64::RSP, X86_64::RBP, X86_64::RA].into_iter()
    }
}
