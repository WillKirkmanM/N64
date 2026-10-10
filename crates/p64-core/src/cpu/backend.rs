use super::{InstrFn, instruction::Instruction, types::PreparedInstruction};
use crate::{N64, recompiler::Backend};

#[inline(always)]
pub(super) fn execute(
    n64: &mut N64,
    opcode: u32,
    fallback: InstrFn,
    prepared: PreparedInstruction,
) {
    if matches!(prepared, PreparedInstruction::Decoded) {
        n64.execution_engine
            .stats
            .record_decoded(opcode, n64.execution_engine.backend);

        fallback(n64, opcode);
        return;
    }

    if n64.execution_engine.backend == Backend::Interpreter {
        fallback(n64, opcode);
        return;
    }

    let wide = Instruction::decode(opcode).native_requires_wide();

    if wide && !n64.cp0().status().wide_instructions_allowed() {
        fallback(n64, opcode);
        return;
    }

    if let PreparedInstruction::Native(native) = prepared {
        if n64
            .execution_engine
            .execute_prepared(native, &mut n64.cpu.registers.gpr)
        {
            return;
        }
    }

    if !n64
        .execution_engine
        .try_execute(opcode, &mut n64.cpu.registers.gpr)
    {
        fallback(n64, opcode);
    }
}
