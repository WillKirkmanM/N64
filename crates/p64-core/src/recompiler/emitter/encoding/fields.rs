const REGISTER_CODE_MASK: u8 = 0x07;
const REGISTER_EXTENSION_SHIFT: u32 = 3;
const MODRM_REGISTER_SHIFT: u32 = 3;
const REX_WIDE: u8 = 0x48;
const REX_REGISTER_SHIFT: u32 = 2;

pub(in crate::recompiler::emitter) const REGISTER_MODE: u8 = 0xc0;
pub(in crate::recompiler::emitter) const DISPLACEMENT_MODE: u8 = 0x80;
pub(in crate::recompiler::emitter) const GPR_BASE_REGISTER: u8 = 2; // RDX

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
