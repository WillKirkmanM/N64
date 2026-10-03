use super::{DpcReg, DpcStatus};
use crate::builder::registers::Registers;

pub struct RdpRegisters {
    pub regs_dpc: Registers,
    pub regs_dps: Registers,
}

impl Default for RdpRegisters {
    fn default() -> Self {
        let mut registers = Self {
            regs_dpc: Registers::default()
                .with_size(DpcReg::RegsCount as usize)
                .build(),
            regs_dps: Registers::default()
                .with_size(DpcReg::RegsCount as usize)
                .build(),
        };
        registers.regs_dpc[DpcReg::Status as usize] =
            (DpcStatus::START_GCLK | DpcStatus::PIPE_BUSY | DpcStatus::CBUF_READY).bits();

        registers
    }
}
