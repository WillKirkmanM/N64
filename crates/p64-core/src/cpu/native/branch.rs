use super::{
    is_scalar_idle_branch, native_branch_target, native_retirement_budget,
    retire_native_integer_instructions,
};
use crate::{recompiler::NativeBranchBlock, N64};

pub(super) struct ExecutedBranch {
    pub(super) taken: bool,
    pub(super) target: u64,
}

#[inline]
pub(super) fn execute_prefix(
    n64: &mut N64,
    block: NativeBranchBlock,
    retirement_length: usize,
) -> Option<ExecutedBranch> {
    let budget = native_retirement_budget(
        n64,
        retirement_length,
        block.first_opcode(),
        block.requires_wide(),
    )?;

    if is_scalar_idle_branch(n64, block) {
        return None;
    }

    let target = native_branch_target(n64.cpu.execution.pc, block.len(), block.branch_opcode());
    let taken = n64
        .execution_engine
        .execute_prepared_branch_block(block, &mut n64.cpu.registers.gpr)?;
    retire_native_integer_instructions(n64, retirement_length, budget);

    Some(ExecutedBranch { taken, target })
}
