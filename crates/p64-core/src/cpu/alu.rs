use super::instruction::{se32, Instruction};
use crate::N64;

const WORD_SHIFT_MASK: u32 = 31;
const DOUBLEWORD_SHIFT_MASK: u32 = 63;
const UPPER_WORD_SHIFT: u32 = 32;

mod operands;
use operands::{
    checked_immediate, checked_registers, immediate, immediate_shift, register_shift, registers,
    unsigned_immediate,
};

fn word_shift_left(value: u64, amount: u32) -> u64 {
    se32(((value as u32) << amount) as i32)
}

fn word_shift_right(value: u64, amount: u32) -> u64 {
    se32(((value as u32) >> amount) as i32)
}

fn word_shift_arithmetic(value: u64, amount: u32) -> u64 {
    // Preserve the full-register shift before sign-extending the low word.
    se32(((value as i64) >> amount) as i32)
}

fn doubleword_shift_left(value: u64, amount: u32) -> u64 {
    value << amount
}

fn doubleword_shift_right(value: u64, amount: u32) -> u64 {
    value >> amount
}

fn doubleword_shift_arithmetic(value: u64, amount: u32) -> u64 {
    ((value as i64) >> amount) as u64
}

fn checked_word_add(lhs: u64, rhs: u64) -> Option<u64> {
    (lhs as i32).checked_add(rhs as i32).map(se32)
}

fn wrapping_word_add(lhs: u64, rhs: u64) -> u64 {
    se32((lhs as i32).wrapping_add(rhs as i32))
}

pub(super) fn addi(n64: &mut N64, opcode: u32) {
    checked_immediate(n64, opcode, |lhs, rhs| checked_word_add(lhs, rhs as u64));
}

pub(super) fn addiu(n64: &mut N64, opcode: u32) {
    immediate(n64, opcode, |lhs, rhs| wrapping_word_add(lhs, rhs as u64));
}

pub(super) fn slti(n64: &mut N64, opcode: u32) {
    immediate(n64, opcode, |lhs, rhs| u64::from((lhs as i64) < rhs));
}

pub(super) fn sltiu(n64: &mut N64, opcode: u32) {
    immediate(n64, opcode, |lhs, rhs| u64::from(lhs < rhs as u64));
}

pub(super) fn andi(n64: &mut N64, opcode: u32) {
    unsigned_immediate(n64, opcode, |lhs, rhs| lhs & rhs);
}

pub(super) fn ori(n64: &mut N64, opcode: u32) {
    unsigned_immediate(n64, opcode, |lhs, rhs| lhs | rhs);
}

pub(super) fn xori(n64: &mut N64, opcode: u32) {
    unsigned_immediate(n64, opcode, |lhs, rhs| lhs ^ rhs);
}

pub(super) fn daddi(n64: &mut N64, opcode: u32) {
    checked_immediate(n64, opcode, |lhs, rhs| {
        (lhs as i64).checked_add(rhs).map(|value| value as u64)
    });
}

pub(super) fn daddiu(n64: &mut N64, opcode: u32) {
    immediate(n64, opcode, |lhs, rhs| lhs.wrapping_add(rhs as u64));
}

pub(super) fn addu(n64: &mut N64, opcode: u32) {
    registers(n64, opcode, wrapping_word_add);
}

pub(super) fn subu(n64: &mut N64, opcode: u32) {
    registers(n64, opcode, |lhs, rhs| {
        se32((lhs as i32).wrapping_sub(rhs as i32))
    });
}

pub(super) fn add(n64: &mut N64, opcode: u32) {
    checked_registers(n64, opcode, checked_word_add);
}

pub(super) fn sub(n64: &mut N64, opcode: u32) {
    checked_registers(n64, opcode, |lhs, rhs| {
        (lhs as i32).checked_sub(rhs as i32).map(se32)
    });
}

pub(super) fn dadd(n64: &mut N64, opcode: u32) {
    checked_registers(n64, opcode, |lhs, rhs| {
        (lhs as i64)
            .checked_add(rhs as i64)
            .map(|value| value as u64)
    });
}

pub(super) fn dsub(n64: &mut N64, opcode: u32) {
    checked_registers(n64, opcode, |lhs, rhs| {
        (lhs as i64)
            .checked_sub(rhs as i64)
            .map(|value| value as u64)
    });
}

pub(super) fn and(n64: &mut N64, opcode: u32) {
    registers(n64, opcode, |lhs, rhs| lhs & rhs);
}

pub(super) fn or(n64: &mut N64, opcode: u32) {
    registers(n64, opcode, |lhs, rhs| lhs | rhs);
}

pub(super) fn xor(n64: &mut N64, opcode: u32) {
    registers(n64, opcode, |lhs, rhs| lhs ^ rhs);
}

pub(super) fn nor(n64: &mut N64, opcode: u32) {
    registers(n64, opcode, |lhs, rhs| !(lhs | rhs));
}

pub(super) fn slt(n64: &mut N64, opcode: u32) {
    registers(n64, opcode, |lhs, rhs| {
        u64::from((lhs as i64) < (rhs as i64))
    });
}

pub(super) fn sltu(n64: &mut N64, opcode: u32) {
    registers(n64, opcode, |lhs, rhs| u64::from(lhs < rhs));
}

pub(super) fn daddu(n64: &mut N64, opcode: u32) {
    registers(n64, opcode, u64::wrapping_add);
}

pub(super) fn dsubu(n64: &mut N64, opcode: u32) {
    registers(n64, opcode, u64::wrapping_sub);
}

pub(super) fn lui(n64: &mut N64, opcode: u32) {
    let instruction = Instruction::decode(opcode);
    n64.cpu_mut().set_gpr(
        instruction.rt,
        se32((instruction.immediate_unsigned() as i32) << 16),
    );
}

pub(super) fn sll(n64: &mut N64, opcode: u32) {
    immediate_shift(n64, opcode, 0, word_shift_left);
}

pub(super) fn srl(n64: &mut N64, opcode: u32) {
    immediate_shift(n64, opcode, 0, word_shift_right);
}

pub(super) fn sra(n64: &mut N64, opcode: u32) {
    immediate_shift(n64, opcode, 0, word_shift_arithmetic);
}

pub(super) fn sllv(n64: &mut N64, opcode: u32) {
    register_shift(n64, opcode, WORD_SHIFT_MASK, word_shift_left);
}

pub(super) fn srlv(n64: &mut N64, opcode: u32) {
    register_shift(n64, opcode, WORD_SHIFT_MASK, word_shift_right);
}

pub(super) fn srav(n64: &mut N64, opcode: u32) {
    register_shift(n64, opcode, WORD_SHIFT_MASK, word_shift_arithmetic);
}

pub(super) fn dsllv(n64: &mut N64, opcode: u32) {
    register_shift(n64, opcode, DOUBLEWORD_SHIFT_MASK, doubleword_shift_left);
}

pub(super) fn dsrlv(n64: &mut N64, opcode: u32) {
    register_shift(n64, opcode, DOUBLEWORD_SHIFT_MASK, doubleword_shift_right);
}

pub(super) fn dsrav(n64: &mut N64, opcode: u32) {
    register_shift(
        n64,
        opcode,
        DOUBLEWORD_SHIFT_MASK,
        doubleword_shift_arithmetic,
    );
}

pub(super) fn dsll(n64: &mut N64, opcode: u32) {
    immediate_shift(n64, opcode, 0, doubleword_shift_left);
}

pub(super) fn dsrl(n64: &mut N64, opcode: u32) {
    immediate_shift(n64, opcode, 0, doubleword_shift_right);
}

pub(super) fn dsra(n64: &mut N64, opcode: u32) {
    immediate_shift(n64, opcode, 0, doubleword_shift_arithmetic);
}

pub(super) fn dsll32(n64: &mut N64, opcode: u32) {
    immediate_shift(n64, opcode, UPPER_WORD_SHIFT, doubleword_shift_left);
}

pub(super) fn dsrl32(n64: &mut N64, opcode: u32) {
    immediate_shift(n64, opcode, UPPER_WORD_SHIFT, doubleword_shift_right);
}

pub(super) fn dsra32(n64: &mut N64, opcode: u32) {
    immediate_shift(n64, opcode, UPPER_WORD_SHIFT, doubleword_shift_arithmetic);
}
