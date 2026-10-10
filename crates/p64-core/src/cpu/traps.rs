use super::Instruction;
use crate::{exceptions, mmu::add_cycles, N64};

const TRAP_EXCEPTION_CYCLES: u64 = 2;

mod types;
use types::TrapCondition;
use TrapCondition::*;

fn trap_if(n64: &mut N64, condition: TrapCondition, source: u64, target: u64) {
    if condition.matches(source, target) {
        add_cycles(n64, TRAP_EXCEPTION_CYCLES);
        exceptions::trap_exception(n64);
    }
}

fn trap_register(n64: &mut N64, opcode: u32, condition: TrapCondition) {
    let instruction = Instruction::decode(opcode);
    let cpu = n64.cpu();
    let source = cpu.gpr(instruction.rs);
    let target = cpu.gpr(instruction.rt);
    trap_if(n64, condition, source, target);
}

fn trap_immediate(n64: &mut N64, opcode: u32, condition: TrapCondition) {
    let instruction = Instruction::decode(opcode);
    let source = n64.cpu().gpr(instruction.rs);
    // Both signed and unsigned immediate traps sign-extend the encoded 16-bit operand.
    let target = instruction.immediate_signed() as u64;
    trap_if(n64, condition, source, target);
}

pub(super) fn tge(n64: &mut N64, opcode: u32) {
    trap_register(n64, opcode, SignedGreaterOrEqual);
}

pub(super) fn tgeu(n64: &mut N64, opcode: u32) {
    trap_register(n64, opcode, UnsignedGreaterOrEqual);
}

pub(super) fn tlt(n64: &mut N64, opcode: u32) {
    trap_register(n64, opcode, SignedLess);
}

pub(super) fn tltu(n64: &mut N64, opcode: u32) {
    trap_register(n64, opcode, UnsignedLess);
}

pub(super) fn teq(n64: &mut N64, opcode: u32) {
    trap_register(n64, opcode, Equal);
}

pub(super) fn tne(n64: &mut N64, opcode: u32) {
    trap_register(n64, opcode, NotEqual);
}

pub(super) fn tgei(n64: &mut N64, opcode: u32) {
    trap_immediate(n64, opcode, SignedGreaterOrEqual);
}

pub(super) fn tgeiu(n64: &mut N64, opcode: u32) {
    trap_immediate(n64, opcode, UnsignedGreaterOrEqual);
}

pub(super) fn tlti(n64: &mut N64, opcode: u32) {
    trap_immediate(n64, opcode, SignedLess);
}

pub(super) fn tltiu(n64: &mut N64, opcode: u32) {
    trap_immediate(n64, opcode, UnsignedLess);
}

pub(super) fn teqi(n64: &mut N64, opcode: u32) {
    trap_immediate(n64, opcode, Equal);
}

pub(super) fn tnei(n64: &mut N64, opcode: u32) {
    trap_immediate(n64, opcode, NotEqual);
}
