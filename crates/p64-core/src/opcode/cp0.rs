//! CO operand bits do not select the operation.

const COPROCESSOR_ENCODING_COUNT: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Cp0Opcode {
    Mfc0 = 0x00,
    Dmfc0 = 0x01,
    Cfc0 = 0x02,
    Mtc0 = 0x04,
    Dmtc0 = 0x05,
    Ctc0 = 0x06,
    Bc0 = 0x08,
    Co = 0x10,
}

impl Cp0Opcode {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Cp0Function {
    Tlbr = 0x01,
    Tlbwi = 0x02,
    Tlbwr = 0x06,
    Tlbp = 0x08,
    Rfe = 0x10,
    Eret = 0x18,
}

impl Cp0Function {}
