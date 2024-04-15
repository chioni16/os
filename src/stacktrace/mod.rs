#[cfg(feature = "eh_frame")]
mod eh_frame;
#[cfg(feature = "eh_frame")]
pub use eh_frame::unwind;

#[cfg(not(feature = "eh_frame"))]
mod frame_pointer;
#[cfg(not(feature = "eh_frame"))]
pub use frame_pointer::unwind;

use gimli::{Register, X86_64};
use crate::mem::VirtualAddress;

#[derive(Debug)]
pub struct CallFrame {
    pub pc: VirtualAddress,
    // pub symbol: Option<&'static str>,
    // pub sym_off: Option<usize>,
    // pub file_line: Option<(&'static str, u32)>,
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