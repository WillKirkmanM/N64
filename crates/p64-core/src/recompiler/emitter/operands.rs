use crate::opcode::{PrimaryOpcode, SpecialOpcode};
#[derive(Clone, Copy)]
pub(super) struct Operands {
    pub(super) primary: Option<PrimaryOpcode>,
    pub(super) function: Option<SpecialOpcode>,
    pub(super) rs: usize,
    pub(super) rt: usize,
    pub(super) rd: usize,
    pub(super) shift: u8,
}

impl Operands {
    pub(super) fn decode(opcode: u32) -> Self {
        let instruction = crate::cpu::Instruction::decode(opcode);

        Self {
            primary: PrimaryOpcode::from_encoding(instruction.primary() as usize),
            function: SpecialOpcode::from_encoding(instruction.function()),
            rs: instruction.rs,
            rt: instruction.rt,
            rd: instruction.rd,
            shift: instruction.shift_amount() as u8,
        }
    }
}
