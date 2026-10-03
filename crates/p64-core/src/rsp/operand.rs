use crate::cpu::rt;
pub(super) use crate::cpu::{imm, rd, rs};

const SHIFT_AMOUNT_SHIFT: u32 = 6;
const REGISTER_INDEX_MASK: u32 = 31;
const VECTOR_ELEMENT_MASK: u32 = 15;
const VECTOR_MEMORY_ELEMENT_SHIFT: u32 = 7;
const VECTOR_MEMORY_OFFSET_MASK: u8 = 0x7f;
const DIVISION_ELEMENT_MASK: u32 = 7;

#[inline(always)]
pub(super) fn sa(opcode: u32) -> u32 {
    (opcode >> SHIFT_AMOUNT_SHIFT) & REGISTER_INDEX_MASK
}

#[inline(always)]
pub(super) fn voffset(opcode: u32) -> u8 {
    opcode as u8 & VECTOR_MEMORY_OFFSET_MASK
}

#[inline(always)]
pub(super) fn velement(opcode: u32) -> u8 {
    ((opcode >> VECTOR_MEMORY_ELEMENT_SHIFT) & VECTOR_ELEMENT_MASK) as u8
}

#[inline(always)]
pub(super) fn vt(opcode: u32) -> u32 {
    rt(opcode)
}

#[inline(always)]
pub(super) fn vs(opcode: u32) -> u32 {
    rd(opcode)
}

#[inline(always)]
pub(super) fn vd(opcode: u32) -> u32 {
    sa(opcode)
}

#[inline(always)]
pub(super) fn ve(opcode: u32) -> u32 {
    rs(opcode) & VECTOR_ELEMENT_MASK
}

#[inline(always)]
pub(super) fn de(opcode: u32) -> u32 {
    rd(opcode) & DIVISION_ELEMENT_MASK
}
