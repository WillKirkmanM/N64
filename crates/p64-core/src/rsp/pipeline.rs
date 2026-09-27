use crate::{
    cpu::Instruction,
    opcode::{Cp0Opcode, PrimaryOpcode, RegImmOpcode, RspCop2Opcode, SpecialOpcode},
};

const COP0_RESULT_LATENCY: u64 = 3;
const ZERO_REGISTER_MASK: u32 = 1;

#[derive(Default)]
pub(super) struct Cop0Readiness {
    tick: u64,
    ready_ticks: [u64; 32],
    latest_ready_tick: u64,
}

impl Cop0Readiness {
    pub(super) fn record(&mut self, register: usize) {
        if register != 0 {
            self.ready_ticks[register] = self.tick + COP0_RESULT_LATENCY;
            self.latest_ready_tick = self.latest_ready_tick.max(self.ready_ticks[register]);
        }
    }

    pub(super) fn prepare(&mut self, opcode: u32) -> u64 {
        if self.tick >= self.latest_ready_tick {
            return 0;
        }
        let cycles = self.stall(opcode);
        self.overwrite(opcode);
        cycles
    }

    fn stall(&mut self, opcode: u32) -> u64 {
        let mut sources = scalar_sources(opcode);
        let mut ready = self.tick;
        while sources != 0 {
            let register = sources.trailing_zeros() as usize;
            ready = ready.max(self.ready_ticks[register]);
            sources &= sources - 1;
        }
        let cycles = ready - self.tick;
        self.advance(cycles);
        cycles
    }

    pub(super) fn advance(&mut self, cycles: u64) {
        self.tick += cycles;
    }

    fn overwrite(&mut self, opcode: u32) {
        if let Some(register) = scalar_destination(opcode) {
            self.ready_ticks[register] = self.tick;
        }
    }
}

fn scalar_sources(opcode: u32) -> u32 {
    let instruction = Instruction::decode(opcode);
    let source = 1 << instruction.rs;
    let target = 1 << instruction.rt;
    use PrimaryOpcode::*;
    let mask = match PrimaryOpcode::from_encoding((opcode >> 26) as usize) {
        Some(Special) => match SpecialOpcode::from_encoding(instruction.function()) {
            Some(SpecialOpcode::Sll | SpecialOpcode::Srl | SpecialOpcode::Sra) => target,
            Some(SpecialOpcode::Jr | SpecialOpcode::Jalr) => source,
            Some(SpecialOpcode::Break) => 0,
            Some(
                SpecialOpcode::Sllv
                | SpecialOpcode::Srlv
                | SpecialOpcode::Srav
                | SpecialOpcode::Add
                | SpecialOpcode::Addu
                | SpecialOpcode::Sub
                | SpecialOpcode::Subu
                | SpecialOpcode::And
                | SpecialOpcode::Or
                | SpecialOpcode::Xor
                | SpecialOpcode::Nor
                | SpecialOpcode::Slt
                | SpecialOpcode::Sltu,
            ) => source | target,
            _ => source,
        },
        Some(Beq | Bne | Sb | Sh | Sw) => source | target,
        Some(
            Blez | Bgtz | Addi | Addiu | Slti | Sltiu | Andi | Ori | Xori | Lb | Lh | Lw | Lbu
            | Lhu | Lwu | Lwc2 | Swc2,
        ) => source,
        Some(RegImm) => match RegImmOpcode::from_encoding(instruction.rt) {
            Some(
                RegImmOpcode::Bltz
                | RegImmOpcode::Bgez
                | RegImmOpcode::Bltzal
                | RegImmOpcode::Bgezal,
            ) => source,
            _ => 0,
        },
        Some(Cop0) => match Cp0Opcode::from_encoding(instruction.rs) {
            Some(Cp0Opcode::Mtc0) => target,
            _ => 0,
        },
        Some(Cop2) => match RspCop2Opcode::from_encoding(instruction.rs) {
            Some(RspCop2Opcode::Mtc2 | RspCop2Opcode::Ctc2) => target,
            _ => 0,
        },
        _ => 0,
    };
    mask & !ZERO_REGISTER_MASK
}

fn scalar_destination(opcode: u32) -> Option<usize> {
    let instruction = Instruction::decode(opcode);
    use PrimaryOpcode::*;
    match PrimaryOpcode::from_encoding((opcode >> 26) as usize) {
        Some(Special) => match SpecialOpcode::from_encoding(instruction.function()) {
            Some(SpecialOpcode::Jr | SpecialOpcode::Break) => None,
            _ => Some(instruction.rd),
        },
        Some(Jal) => Some(31),
        Some(RegImm) => match RegImmOpcode::from_encoding(instruction.rt) {
            Some(RegImmOpcode::Bltzal | RegImmOpcode::Bgezal) => Some(31),
            _ => None,
        },
        Some(
            Addi | Addiu | Slti | Sltiu | Andi | Ori | Xori | Lui | Lb | Lh | Lw | Lbu | Lhu | Lwu,
        ) => Some(instruction.rt),
        Some(Cop0) => match Cp0Opcode::from_encoding(instruction.rs) {
            Some(Cp0Opcode::Mfc0) => Some(instruction.rt),
            _ => None,
        },
        Some(Cop2) => match RspCop2Opcode::from_encoding(instruction.rs) {
            Some(RspCop2Opcode::Mfc2 | RspCop2Opcode::Cfc2) => Some(instruction.rt),
            _ => None,
        },
        _ => None,
    }
}
