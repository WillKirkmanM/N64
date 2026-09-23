use super::*;

/// Reality Signal Processor (RSP) - Nintendo 64's vector processing unit
///
/// General register state
pub struct RegisterState {
    pub regs: Registers,
    pub regs2: Registers,
    pub gpr: [u32; 32],
    pub vpr: [u128; 32],
}

#[repr(u32)]
pub enum SpReg {
    MemAddr = 0,
    DramAddr = 1,
    RdLen = 2,
    WrLen = 3,
    Status = 4,
    DmaFull = 5,
    DmaBusy = 6,
    Semaphore = 7,
    RegsCount = 8,
}

#[repr(u32)]
pub enum SpReg2 {
    Pc = 0,
    // Ibist = 1,
    Regs2Count = 2,
}

#[repr(u32)]
pub enum SpStatus {
    Halt = 1 << 0,
    Broke = 1 << 1,
    DmaBusy = 1 << 2,
    DmaFull = 1 << 3,
    // IoFull = 1 << 4,
    Sstep = 1 << 5,
    IntrBreak = 1 << 6,
    Sig0 = 1 << 7,
    Sig1 = 1 << 8,
    Sig2 = 1 << 9,
    Sig3 = 1 << 10,
    Sig4 = 1 << 11,
    Sig5 = 1 << 12,
    Sig6 = 1 << 13,
    Sig7 = 1 << 14,
}

#[repr(u32)]
pub enum SpStatusWrite {
    ClrHalt = 1 << 0,
    SetHalt = 1 << 1,
    ClrBroke = 1 << 2,
    ClrIntr = 1 << 3,
    SetIntr = 1 << 4,
    ClrSstep = 1 << 5,
    SetSstep = 1 << 6,
    ClrIntrBreak = 1 << 7,
    SetIntrBreak = 1 << 8,
    ClrSig0 = 1 << 9,
    SetSig0 = 1 << 10,
    ClrSig1 = 1 << 11,
    SetSig1 = 1 << 12,
    ClrSig2 = 1 << 13,
    SetSig2 = 1 << 14,
    ClrSig3 = 1 << 15,
    SetSig3 = 1 << 16,
    ClrSig4 = 1 << 17,
    SetSig4 = 1 << 18,
    ClrSig5 = 1 << 19,
    SetSig5 = 1 << 20,
    ClrSig6 = 1 << 21,
    SetSig6 = 1 << 22,
    ClrSig7 = 1 << 23,
    SetSig7 = 1 << 24,
}

impl Default for RegisterState {
    fn default() -> Self {
        Self {
            regs: Registers::new()
                .with_size(SpReg::RegsCount as usize)
                .build(),
            regs2: Registers::new()
                .with_size(SpReg2::Regs2Count as usize)
                .build(),
            gpr: [0; 32],
            vpr: [0; 32],
        }
    }
}
