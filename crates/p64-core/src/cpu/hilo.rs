use super::{instruction::Instruction, Cpu};
use crate::{mmu::add_cycles, N64};

mod result;
use result::HiLo;

const WORD_MULTIPLY_CYCLES: u64 = 4;
const DOUBLEWORD_MULTIPLY_CYCLES: u64 = 7;
const WORD_DIVIDE_CYCLES: u64 = 36;
const DOUBLEWORD_DIVIDE_CYCLES: u64 = 68;
const SIGNED_MULTIPLIER_EXTENSION_SHIFT: u32 = 29;

fn calculate(n64: &mut N64, opcode: u32, cycles: u64, operation: impl FnOnce(u64, u64) -> HiLo) {
    let instruction = Instruction::decode(opcode);
    let cpu = n64.cpu_mut();
    operation(cpu.gpr(instruction.rs), cpu.gpr(instruction.rt)).write(cpu);
    add_cycles(n64, cycles);
}

fn read_accumulator(cpu: &mut Cpu, instruction: Instruction, read: impl FnOnce(&Cpu) -> u64) {
    let value = read(cpu);
    cpu.set_gpr(instruction.rd, value);
}

fn write_accumulator(cpu: &mut Cpu, instruction: Instruction, write: impl FnOnce(&mut Cpu, u64)) {
    let value = cpu.gpr(instruction.rs);
    write(cpu, value);
}

pub(super) fn sync(_n64: &mut N64, _opcode: u32) {}

pub(super) fn mfhi(n64: &mut N64, opcode: u32) {
    read_accumulator(n64.cpu_mut(), Instruction::decode(opcode), |cpu| {
        cpu.registers.hi
    });
}

pub(super) fn mflo(n64: &mut N64, opcode: u32) {
    read_accumulator(n64.cpu_mut(), Instruction::decode(opcode), |cpu| {
        cpu.registers.lo
    });
}

pub(super) fn mthi(n64: &mut N64, opcode: u32) {
    write_accumulator(n64.cpu_mut(), Instruction::decode(opcode), |cpu, value| {
        cpu.registers.hi = value
    });
}

pub(super) fn mtlo(n64: &mut N64, opcode: u32) {
    write_accumulator(n64.cpu_mut(), Instruction::decode(opcode), |cpu, value| {
        cpu.registers.lo = value
    });
}

pub(super) fn mult(n64: &mut N64, opcode: u32) {
    calculate(n64, opcode, WORD_MULTIPLY_CYCLES, |lhs, rhs| {
        // VR4300 word multiply consumes all 64 rs bits and a signed 35-bit rt.
        let rhs = ((rhs << SIGNED_MULTIPLIER_EXTENSION_SHIFT) as i64)
            >> SIGNED_MULTIPLIER_EXTENSION_SHIFT;

        HiLo::word_product((lhs as i64).wrapping_mul(rhs) as u64)
    });
}

pub(super) fn multu(n64: &mut N64, opcode: u32) {
    calculate(n64, opcode, WORD_MULTIPLY_CYCLES, |lhs, rhs| {
        HiLo::word_product(u64::from(lhs as u32) * u64::from(rhs as u32))
    });
}

pub(super) fn dmult(n64: &mut N64, opcode: u32) {
    calculate(n64, opcode, DOUBLEWORD_MULTIPLY_CYCLES, |lhs, rhs| {
        HiLo::product(((lhs as i64 as i128) * (rhs as i64 as i128)) as u128)
    });
}

pub(super) fn dmultu(n64: &mut N64, opcode: u32) {
    calculate(n64, opcode, DOUBLEWORD_MULTIPLY_CYCLES, |lhs, rhs| {
        HiLo::product(u128::from(lhs) * u128::from(rhs))
    });
}

pub(super) fn div(n64: &mut N64, opcode: u32) {
    calculate(n64, opcode, WORD_DIVIDE_CYCLES, |lhs, rhs| {
        HiLo::signed_division(lhs as i32 as i64, rhs as i32 as i64).sign_extend_words()
    });
}

pub(super) fn divu(n64: &mut N64, opcode: u32) {
    calculate(n64, opcode, WORD_DIVIDE_CYCLES, |lhs, rhs| {
        HiLo::unsigned_division(u64::from(lhs as u32), u64::from(rhs as u32)).sign_extend_words()
    });
}

pub(super) fn ddiv(n64: &mut N64, opcode: u32) {
    calculate(n64, opcode, DOUBLEWORD_DIVIDE_CYCLES, |lhs, rhs| {
        HiLo::signed_division(lhs as i64, rhs as i64)
    });
}

pub(super) fn ddivu(n64: &mut N64, opcode: u32) {
    calculate(
        n64,
        opcode,
        DOUBLEWORD_DIVIDE_CYCLES,
        HiLo::unsigned_division,
    );
}
