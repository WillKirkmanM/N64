const REGISTER_CODE_MASK: u8 = 0x07;
const REGISTER_EXTENSION_SHIFT: u32 = 3;
const MODRM_REGISTER_SHIFT: u32 = 3;
const REX_WIDE: u8 = 0x48;
const REX_REGISTER_SHIFT: u32 = 2;

pub(in crate::recompiler::emitter) const REGISTER_MODE: u8 = 0xc0;
pub(in crate::recompiler::emitter) const DISPLACEMENT_MODE: u8 = 0x80;
pub(in crate::recompiler::emitter) const GPR_BASE_REGISTER: u8 = 2; // RDX

pub(in crate::recompiler::emitter) fn wide_rex(register: u8, base: u8) -> u8 {
    REX_WIDE
        | ((register >> REGISTER_EXTENSION_SHIFT) << REX_REGISTER_SHIFT)
        | (base >> REGISTER_EXTENSION_SHIFT)
}

pub(in crate::recompiler::emitter) fn modrm(mode: u8, register: u8, base: u8) -> u8 {
    mode | ((register & REGISTER_CODE_MASK) << MODRM_REGISTER_SHIFT) | (base & REGISTER_CODE_MASK)
}

/// Group-1 arithmetic selector placed in the ModR/M register field.
#[derive(Clone, Copy)]
#[repr(u8)]
pub(in crate::recompiler::emitter) enum ArithmeticOperation {
    Add = 0,
    Or = 1,
    And = 4,
    Xor = 6,
    Compare = 7,
}
