use super::{in_delay_slot, in_delay_slot_taken, Instruction};
use crate::{memory, mmu::MmuRegister, N64};

const LINK_REGISTER: usize = 31;
const JUMP_INDEX_MASK: u32 = 0x03ff_ffff;
const PHYSICAL_JUMP_REGION_MASK: u64 = 0x0fff_ffff;
const IDLE_LOOP_OFFSET: i16 = -1;
const INSTRUCTION_BYTES: u64 = 4;
const LINK_OFFSET: u64 = INSTRUCTION_BYTES * 2;

mod types;
use types::{BranchCondition, DelaySlotPolicy};
use BranchCondition::*;
use DelaySlotPolicy::*;

/// Advance to the next event only when the loop's delay slot is a native-endian NOP.
pub fn check_idle_loop(n64: &mut N64, looping: bool) {
    if !looping || memory::reverse_endian(n64) {
        return;
    }

    let physical_pc = n64.cpu().execution.pc_phys;
    let read = n64.memory.bus.fast_read[(physical_pc >> 16) as usize];

    if read(
        n64,
        physical_pc + INSTRUCTION_BYTES,
        memory::AccessSize::Word,
    ) == 0
    {
        n64.set_cp0_register(MmuRegister::Count, n64.cpu().scheduler.next_event_count);
    }
}

pub fn check_relative_idle_loop(n64: &mut N64, opcode: u32) {
    let instruction = Instruction::decode(opcode);
    check_idle_loop(
        n64,
        instruction.immediate_signed() == i64::from(IDLE_LOOP_OFFSET),
    );
}

pub(super) fn check_absolute_idle_loop(n64: &mut N64, opcode: u32) {
    let target = (opcode & JUMP_INDEX_MASK) as u64;
    let current = (n64.cpu().execution.pc_phys & PHYSICAL_JUMP_REGION_MASK) >> 2;
    check_idle_loop(n64, target == current);
}

fn jump_link_address(n64: &N64) -> u64 {
    if in_delay_slot_taken(n64) {
        n64.cpu().branch_state.pc.wrapping_add(INSTRUCTION_BYTES)
    } else {
        n64.cpu().execution.pc.wrapping_add(LINK_OFFSET)
    }
}

fn jump_absolute(n64: &mut N64, opcode: u32) {
    if in_delay_slot_taken(n64) {
        return;
    }

    check_absolute_idle_loop(n64, opcode);
    let target = Instruction::decode(opcode).jump_target(n64.cpu().execution.pc);
    n64.cpu_mut().branch_state.take(target);
}

fn jump_register(n64: &mut N64, opcode: u32) {
    if in_delay_slot_taken(n64) {
        return;
    }

    check_relative_idle_loop(n64, opcode);
    let instruction = Instruction::decode(opcode);
    let target = n64.cpu().gpr(instruction.rs);
    n64.cpu_mut().branch_state.take(target);
}

pub(super) fn j(n64: &mut N64, opcode: u32) {
    jump_absolute(n64, opcode);
}

pub(super) fn jal(n64: &mut N64, opcode: u32) {
    // JAL writes its link before checking the delay-slot instruction for an idle loop.
    let link = jump_link_address(n64);
    n64.cpu_mut().set_gpr(LINK_REGISTER, link);
    jump_absolute(n64, opcode);
}

pub(super) fn jr(n64: &mut N64, opcode: u32) {
    jump_register(n64, opcode);
}

pub(super) fn jalr(n64: &mut N64, opcode: u32) {
    // Read the jump source before writing rd: JALR permits rs and rd to overlap.
    jump_register(n64, opcode);
    let instruction = Instruction::decode(opcode);
    let link = jump_link_address(n64);
    n64.cpu_mut().set_gpr(instruction.rd, link);
}

fn branch_relative(n64: &mut N64, opcode: u32, taken: bool, delay_slot: DelaySlotPolicy) {
    // In a taken delay slot, the old target becomes the new branch's delay-slot PC.
    let was_delay_slot = in_delay_slot(n64);

    if in_delay_slot_taken(n64) {
        n64.cpu_mut().execution.pc = n64.cpu().branch_state.pc.wrapping_sub(INSTRUCTION_BYTES);
    }

    if !taken {
        n64.cpu_mut().branch_state.decline(delay_slot);
        return;
    }

    if !was_delay_slot {
        check_relative_idle_loop(n64, opcode);
    }

    let target = Instruction::decode(opcode).branch_target(n64.cpu().execution.pc);
    n64.cpu_mut().branch_state.take(target);
}

fn branch(n64: &mut N64, opcode: u32, condition: BranchCondition, delay_slot: DelaySlotPolicy) {
    let instruction = Instruction::decode(opcode);
    let cpu = n64.cpu();
    let taken = condition.matches(cpu.gpr(instruction.rs), cpu.gpr(instruction.rt));
    branch_relative(n64, opcode, taken, delay_slot);
}

fn branch_and_link(
    n64: &mut N64,
    opcode: u32,
    condition: BranchCondition,
    delay_slot: DelaySlotPolicy,
) {
    let link = if in_delay_slot_taken(n64) {
        n64.cpu().branch_state.pc.wrapping_add(INSTRUCTION_BYTES)
    } else {
        n64.cpu().execution.pc.wrapping_add(LINK_OFFSET)
    };
    // Evaluate rs and perform the branch before writing r31, even on an untaken branch.
    branch(n64, opcode, condition, delay_slot);
    n64.cpu_mut().set_gpr(LINK_REGISTER, link);
}

pub(super) fn beq(n64: &mut N64, opcode: u32) {
    branch(n64, opcode, Equal, Execute);
}

pub(super) fn bne(n64: &mut N64, opcode: u32) {
    branch(n64, opcode, NotEqual, Execute);
}

pub(super) fn blez(n64: &mut N64, opcode: u32) {
    branch(n64, opcode, LessOrEqualZero, Execute);
}

pub(super) fn bgtz(n64: &mut N64, opcode: u32) {
    branch(n64, opcode, GreaterZero, Execute);
}

pub(super) fn beql(n64: &mut N64, opcode: u32) {
    branch(n64, opcode, Equal, AnnulIfUntaken);
}

pub(super) fn bnel(n64: &mut N64, opcode: u32) {
    branch(n64, opcode, NotEqual, AnnulIfUntaken);
}

pub(super) fn blezl(n64: &mut N64, opcode: u32) {
    branch(n64, opcode, LessOrEqualZero, AnnulIfUntaken);
}

pub(super) fn bgtzl(n64: &mut N64, opcode: u32) {
    branch(n64, opcode, GreaterZero, AnnulIfUntaken);
}

pub(super) fn bltz(n64: &mut N64, opcode: u32) {
    branch(n64, opcode, Negative, Execute);
}

pub(super) fn bgez(n64: &mut N64, opcode: u32) {
    branch(n64, opcode, Nonnegative, Execute);
}

pub(super) fn bltzl(n64: &mut N64, opcode: u32) {
    branch(n64, opcode, Negative, AnnulIfUntaken);
}

pub(super) fn bgezl(n64: &mut N64, opcode: u32) {
    branch(n64, opcode, Nonnegative, AnnulIfUntaken);
}

pub(super) fn bltzal(n64: &mut N64, opcode: u32) {
    branch_and_link(n64, opcode, Negative, Execute);
}

pub(super) fn bgezal(n64: &mut N64, opcode: u32) {
    branch_and_link(n64, opcode, Nonnegative, Execute);
}

pub(super) fn bltzall(n64: &mut N64, opcode: u32) {
    branch_and_link(n64, opcode, Negative, AnnulIfUntaken);
}

pub(super) fn bgezall(n64: &mut N64, opcode: u32) {
    branch_and_link(n64, opcode, Nonnegative, AnnulIfUntaken);
}
