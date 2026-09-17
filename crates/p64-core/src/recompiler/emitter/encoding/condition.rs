const SET_CONDITION_BASE: u8 = 0x90;
const JUMP_CONDITION_BASE: u8 = 0x80;

#[derive(Clone, Copy)]
#[repr(u8)]
pub(in crate::recompiler::emitter) enum Condition {
    Below = 0x2,
    AboveOrEqual = 0x3,
    Equal = 0x4,
    NotEqual = 0x5,
    Less = 0xc,
    LessOrEqual = 0xe,
    Greater = 0xf,
}

impl Condition {}
