use super::types::NativeRetirementBudget;
use super::{State, gpr_read_mask, imm, se16};
use crate::{N64, mmu::MmuRegister, recompiler::NativeReject};

mod branch;

/// Retire an integer block after event, privilege and pipeline guards pass.
pub fn execute_native_block(n64: &mut N64, block: crate::recompiler::NativeBlock) -> bool {
    let Some(budget) = native_retirement_budget(
        n64,
        block.len(),
        block.first_opcode(),
        block.requires_wide(),
    ) else {
        return false;
    };

    if !n64
        .execution_engine
        .execute_prepared_block(block, &mut n64.cpu.registers.gpr)
    {
        return false;
    }

    retire_native_integer_instructions(n64, block.len(), budget);

    true
}

#[inline(always)]
fn reject_native<T>(n64: &mut N64, reason: NativeReject, result: T) -> T {
    n64.execution_engine.record_native_reject(reason);

    result
}

#[inline]
fn native_retirement_budget(
    n64: &mut N64,
    length: usize,
    first_opcode: u32,
    wide: bool,
) -> Option<NativeRetirementBudget> {
    if !(2..=crate::recompiler::MAX_BLOCK_INSTRUCTIONS).contains(&length) {
        return reject_native(n64, NativeReject::InvalidLength, None);
    }

    if n64.cpu.branch_state.state != State::Step {
        return reject_native(n64, NativeReject::BranchState, None);
    }

    if n64.cpu.execution.pc & 3 != 0 {
        return reject_native(n64, NativeReject::InvalidFetch, None);
    }

    if n64.cp0().clock.pending_wired_reset.is_some() {
        return reject_native(n64, NativeReject::PendingWired, None);
    }

    if wide && !n64.cp0().status().wide_instructions_allowed() {
        return reject_native(n64, NativeReject::WidePrivilege, None);
    }

    let count = n64.cp0_register(MmuRegister::Count);

    if n64
        .cp0()
        .clock
        .count_write_origin
        .is_some_and(|origin| count.saturating_sub(origin) < 2)
    {
        return reject_native(n64, NativeReject::CountWrite, None);
    }

    let random = n64.cp0_register(MmuRegister::Random);
    let wired = n64.cp0_register(MmuRegister::Wired);
    // CP0 exposes a six-bit Random/Wired recurrence, including Wired > 31.
    if wired > 63 || random > 63 {
        return reject_native(n64, NativeReject::RandomRing, None);
    }

    let pending = n64.cpu.execution.pending_gpr_load;
    let interlock = u64::from(pending != 0 && gpr_read_mask(first_opcode) & (1 << pending) != 0);
    let Some(end_count) = count.checked_add(length as u64 + interlock) else {
        return reject_native(n64, NativeReject::Overflow, None);
    };

    if end_count > n64.cpu.scheduler.next_event_count {
        return reject_native(n64, NativeReject::EventBudget, None);
    }

    let Some(end_pc) = n64.cpu.execution.pc.checked_add(length as u64 * 4) else {
        return reject_native(n64, NativeReject::Overflow, None);
    };

    Some(NativeRetirementBudget {
        end_pc,
        end_count,
        random,
        wired,
    })
}

#[inline]
fn retire_native_integer_instructions(
    n64: &mut N64,
    length: usize,
    budget: NativeRetirementBudget,
) {
    n64.cpu.registers.gpr[0] = 0;
    n64.cpu.execution.pending_gpr_load = 0;
    n64.fpu_mut().timing.previous_write = None;
    n64.fpu_mut().timing.pending_write = None;
    n64.cp0_mut().clock.instruction_clock = n64
        .cp0()
        .clock
        .instruction_clock
        .wrapping_add(length as u64);
    n64.set_cp0_register(
        MmuRegister::Random,
        retired_random(budget.random, budget.wired, length as u64),
    );
    n64.set_cp0_register(MmuRegister::Count, budget.end_count);
    n64.cpu.execution.pc = budget.end_pc;
    n64.cpu.execution.data_address = budget.end_pc - 4;
}

#[inline]
fn retired_random(random: u64, wired: u64, length: u64) -> u64 {
    if wired == 0 && random <= 31 {
        return random.wrapping_sub(length) & 31;
    }

    if wired <= 31 && random <= 31 && random >= wired {
        let period = 32 - wired;

        return wired + (random - wired + period - length % period) % period;
    }

    let distance = random.wrapping_sub(wired) & 63;

    if length <= distance {
        return random.wrapping_sub(length) & 63;
    }

    let remaining = length - distance - 1;
    let period = if wired <= 31 { 32 - wired } else { 96 - wired };

    31u64.wrapping_sub(remaining % period) & 63
}

/// Retires cached kernel loads/stores, leaving misses and unsupported accesses to scalar execution.
pub fn execute_native_memory_block(
    n64: &mut N64,
    block: crate::recompiler::NativeMemoryBlock,
) -> usize {
    let status = n64.cp0().status();

    if status.mode() != 0 || status.reverse_endian() {
        return reject_native(n64, NativeReject::MemoryPrivilege, 0);
    }

    let Some(mut budget) = native_retirement_budget(
        n64,
        block.len(),
        block.first_opcode(),
        block.requires_wide(),
    ) else {
        return 0;
    };

    let start_pc = n64.cpu.execution.pc;
    let start_count = n64.cp0_register(MmuRegister::Count);
    // Budget one encoded-source load interlock per instruction before any side effect.
    let Some(worst_count) = start_count.checked_add(block.len() as u64 * 2) else {
        return 0;
    };

    if worst_count > n64.cpu.scheduler.next_event_count {
        return reject_native(n64, NativeReject::EventBudget, 0);
    }

    let original_fpu_pending = n64.fpu().timing.pending_write;
    let (retired, extra_cycles, pending_load, data_address) = {
        let mut context = crate::recompiler::NativeMemoryContext::new(
            &mut n64.cpu.registers.gpr,
            &mut n64.memory.caches.dcache,
            start_pc,
            n64.cpu.execution.pending_gpr_load,
            n64.cpu.execution.data_address,
        );
        let Some(retired) = n64
            .execution_engine
            .execute_prepared_memory_block(block, &mut context)
        else {
            return 0;
        };

        (
            retired,
            context.extra_cycles,
            context.pending_load,
            context.last_data_address,
        )
    };
    budget.end_pc = start_pc + retired as u64 * 4;
    budget.end_count = start_count + retired as u64 + extra_cycles;
    retire_native_integer_instructions(n64, retired, budget);
    n64.cpu.execution.pending_gpr_load = pending_load;
    n64.cpu.execution.data_address = data_address;
    if retired == 1 {
        n64.fpu_mut().timing.previous_write = original_fpu_pending;
    }

    retired
}

pub(crate) fn native_branch_target(start_pc: u64, length: usize, opcode: u32) -> u64 {
    let branch_pc = start_pc.wrapping_add((length as u64 - 1) * 4);

    if opcode >> 26 == 2 {
        (branch_pc.wrapping_add(4) & 0xFFFF_FFFF_F000_0000) | (((opcode & 0x03FF_FFFF) as u64) << 2)
    } else {
        branch_pc
            .wrapping_add(se16(imm(opcode) as i16) << 2)
            .wrapping_add(4)
    }
}

/// Scalar self-branch shortcuts inspect the delay slot and fast-forward Count.
/// They must never be bypassed by either native branch retirement strategy.
#[inline]
fn is_scalar_idle_branch(n64: &N64, block: crate::recompiler::NativeBranchBlock) -> bool {
    let opcode = block.branch_opcode();

    if opcode >> 26 != 2 {
        imm(opcode) as i16 == -1
    } else {
        let branch_physical = n64
            .cpu
            .execution
            .pc_phys
            .wrapping_add((block.len() as u64 - 1) * 4);

        (opcode & 0x03FF_FFFF) as u64 == (branch_physical & 0x0FFF_FFFF) >> 2
    }
}

/// Executes an ALU prefix and branch, leaving delay-slot fetch to the caller.
/// The caller must validate cache and pipeline lookahead first.
pub fn execute_native_branch_block(
    n64: &mut N64,
    block: crate::recompiler::NativeBranchBlock,
) -> bool {
    let Some(branch) = branch::execute_prefix(n64, block, block.len()) else {
        return false;
    };

    let branch::ExecutedBranch { taken, target } = branch;

    if taken {
        n64.cpu.branch_state.pc = target;
    }
    n64.cpu.branch_state.state = if taken {
        State::DelaySlotTaken
    } else {
        State::DelaySlotNotTaken
    };

    true
}

/// Retires a cached NOP delay slot; the caller must validate both fetch paths.
pub(crate) fn execute_native_branch_nop_block(
    n64: &mut N64,
    block: crate::recompiler::NativeBranchBlock,
) -> bool {
    if block.len() >= crate::recompiler::MAX_BLOCK_INSTRUCTIONS {
        return false;
    }

    let length = block.len() + 1;
    let Some(branch) = branch::execute_prefix(n64, block, length) else {
        return false;
    };

    let branch::ExecutedBranch { taken, target } = branch;
    n64.execution_engine
        .stats
        .record_branch_delay_slot(block.len());
    if taken {
        n64.cpu.branch_state.pc = target;
        n64.cpu.execution.pc = target;
    }
    n64.cpu.branch_state.state = State::Step;

    true
}
