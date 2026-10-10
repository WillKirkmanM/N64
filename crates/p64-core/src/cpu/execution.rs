use super::{backend, instruction::Instruction, types::PreparedInstruction, InstrFn, State};
use crate::{fpu, mmu::{self, add_cycles}, N64};

const LOAD_INTERLOCK_CYCLES: u64 = 1;
const CACHE_TAG_LOAD_CYCLES: u64 = 5;
const ANNULLED_SLOT_CYCLES: u64 = 1;

pub fn execute_instruction(n64: &mut N64, opcode: u32, fallback: InstrFn) {
    execute_prepared_instruction(n64, opcode, fallback, None);
}

pub fn execute_prepared_instruction(
    n64: &mut N64,
    opcode: u32,
    fallback: InstrFn,
    prepared: Option<crate::recompiler::NativeInstruction>,
) {
    let prepared = prepared.map_or(PreparedInstruction::Unprepared, PreparedInstruction::Native);
    execute_classified_instruction(n64, opcode, fallback, prepared);
}

#[inline(always)]
pub(crate) fn execute_decoded_instruction(n64: &mut N64, opcode: u32, fallback: InstrFn) {
    execute_classified_instruction(n64, opcode, fallback, PreparedInstruction::Decoded);
}

#[inline(always)]
pub(super) fn execute_classified_instruction(
    n64: &mut N64,
    opcode: u32,
    fallback: InstrFn,
    prepared: PreparedInstruction,
) {
    let instruction = Instruction::decode(opcode);
    begin_instruction(n64, opcode, instruction);
    backend::execute(n64, opcode, fallback, prepared);
    finish_instruction(n64, instruction);
}

fn begin_instruction(n64: &mut N64, opcode: u32, instruction: Instruction) {
    mmu::begin_instruction(n64);
    let pending = std::mem::take(&mut n64.cpu_mut().execution.pending_gpr_load);

    if pending != 0 && instruction.interlock_read_mask() & (1 << pending) != 0 {
        add_cycles(n64, LOAD_INTERLOCK_CYCLES);
    }

    fpu::begin_instruction_timing(n64, opcode);

    if instruction.primary() == 47 && instruction.rt == 5 {
        add_cycles(n64, CACHE_TAG_LOAD_CYCLES);
    }
}

fn finish_instruction(n64: &mut N64, instruction: Instruction) {
    if !matches!(n64.cpu().branch_state.state, State::Exception) {
        if let Some(destination) = instruction.load_destination() {
            n64.cpu_mut().execution.pending_gpr_load = destination;
        }
    }

    if matches!(n64.cpu().branch_state.state, State::Discard) {
        // An annulled likely-branch slot still advances both instruction clocks.
        add_cycles(n64, ANNULLED_SLOT_CYCLES);
        mmu::begin_instruction(n64);
        fpu::begin_instruction_timing(n64, 0);
        n64.cpu_mut().execution.pending_gpr_load = 0;
    }
}
