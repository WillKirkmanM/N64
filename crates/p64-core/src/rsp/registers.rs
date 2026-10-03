use super::{BranchState, CpuState, ADDR_MASK};

const PC_MASK: u32 = 0xFFC;
const JUMP_REGION_MASK: u32 = 0xF000_0000;
const JUMP_TARGET_MASK: u32 = 0x03FF_FFFF;
const JUMP_TARGET_SHIFT: u32 = 2;
use crate::builder::registers::Registers;

pub struct RegisterState {
    pub regs: Registers,
    pub regs2: Registers,
    pub gpr: [u32; 32],
    pub vpr: [u128; 32],
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
    Regs2Count = 2,
}

#[repr(u32)]
pub enum SpStatus {
    Halt = 1 << 0,
    Broke = 1 << 1,
    DmaBusy = 1 << 2,
    DmaFull = 1 << 3,
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

impl RegisterState {
    pub fn scalar_address(&self, opcode: u32) -> u32 {
        self.gpr[super::rs(opcode) as usize].wrapping_add(super::se16(super::imm(opcode) as i16))
    }

    pub fn pc(&self) -> u32 {
        self.regs2[SpReg2::Pc as usize]
    }

    pub(super) fn jump_target(&self, opcode: u32) -> u32 {
        ((self.pc() + 4) & JUMP_REGION_MASK) | ((opcode & JUMP_TARGET_MASK) << JUMP_TARGET_SHIFT)
    }

    pub fn relative_target(&self, opcode: u32) -> u32 {
        self.pc()
            .wrapping_add(super::se16(super::imm(opcode) as i16) << 2)
            .wrapping_add(4)
    }
}

impl RegisterState {
    pub fn vector_address(&self, opcode: u32, scale: u32) -> u32 {
        self.gpr[super::rs(opcode) as usize].wrapping_add(super::sign_extend_7bit_offset(
            super::voffset(opcode),
            scale,
        )) & ADDR_MASK
    }
}

impl RegisterState {
    pub(super) fn advance_pc(&mut self, branch: &mut BranchState) -> bool {
        let pc = &mut self.regs2[SpReg2::Pc as usize];
        let redirected = branch.state == CpuState::DelaySlotTaken;

        match branch.state {
            CpuState::Step => *pc += 4,
            CpuState::Take => {
                *pc += 4;
                branch.state = CpuState::DelaySlotTaken;
            }
            CpuState::NotTaken => {
                *pc += 4;
                branch.state = CpuState::DelaySlotNotTaken;
            }
            CpuState::DelaySlotTaken => {
                *pc = branch.pc;
                branch.state = CpuState::Step;
            }
            CpuState::DelaySlotNotTaken => {
                *pc += 4;
                branch.state = CpuState::Step;
            }
        }
        *pc &= PC_MASK;
        redirected
    }
}

impl RegisterState {
    /// Paired SP commands cancel when both clear and set are asserted.
    pub(super) fn apply_status_command(&mut self, command: u32, clear: u32, set: u32, flag: u32) {
        let status = &mut self.regs[SpReg::Status as usize];

        match (command & clear != 0, command & set != 0) {
            (true, false) => *status &= !flag,
            (false, true) => *status |= flag,
            _ => {}
        }
    }
}
