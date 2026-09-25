use super::*;

#[repr(u32)]
pub enum DpcReg {
    Start = 0,
    End = 1,
    Current = 2,
    Status = 3,
    Clock = 4,
    BufBusy = 5,
    PipeBusy = 6,
    Tmem = 7,
    RegsCount = 8,
}

#[repr(u32)]
pub enum DpsReg {
    RegsCount = 4,
}

pub struct Rdp {
    pub regs_dpc: Registers,
    pub regs_dps: Registers,
    pub wait_frozen: bool,
}

impl Default for Rdp {
    fn default() -> Self {
        let mut rdp = Self {
            regs_dpc: Registers::default()
                .with_size(DpcReg::RegsCount as usize)
                .build(),
            regs_dps: Registers::default()
                .with_size(DpcReg::RegsCount as usize)
                .build(),
            wait_frozen: false,
        };

        // Initialize DPC status register
        let mut status = DpcStatus::from_bits_truncate(rdp.regs_dpc[DpcReg::Status as usize]);
        status.insert(DpcStatus::START_GCLK | DpcStatus::PIPE_BUSY | DpcStatus::CBUF_READY);
        rdp.regs_dpc[DpcReg::Status as usize] = status.bits();

        rdp
    }
}
