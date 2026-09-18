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

    pub(super) fn destination(self) -> usize {
        if self.primary == Some(PrimaryOpcode::Special) {
            self.rd
        } else {
            self.rt
        }
    }

    pub(super) fn is_shift(self) -> bool {
        matches!(
            self.function,
            Some(
                SpecialOpcode::Sll
                    | SpecialOpcode::Srl
                    | SpecialOpcode::Sra
                    | SpecialOpcode::Sllv
                    | SpecialOpcode::Srlv
                    | SpecialOpcode::Srav
                    | SpecialOpcode::Dsllv
                    | SpecialOpcode::Dsrlv
                    | SpecialOpcode::Dsrav
                    | SpecialOpcode::Dsll
                    | SpecialOpcode::Dsrl
                    | SpecialOpcode::Dsra
                    | SpecialOpcode::Dsll32
                    | SpecialOpcode::Dsrl32
                    | SpecialOpcode::Dsra32
            )
        )
    }

    pub(super) fn is_variable_shift(self) -> bool {
        matches!(
            self.function,
            Some(
                SpecialOpcode::Sllv
                    | SpecialOpcode::Srlv
                    | SpecialOpcode::Srav
                    | SpecialOpcode::Dsllv
                    | SpecialOpcode::Dsrlv
                    | SpecialOpcode::Dsrav
            )
        )
    }

    pub(super) fn is_wide_shift(self) -> bool {
        matches!(
            self.function,
            Some(
                SpecialOpcode::Dsllv
                    | SpecialOpcode::Dsrlv
                    | SpecialOpcode::Dsrav
                    | SpecialOpcode::Dsll
                    | SpecialOpcode::Dsrl
                    | SpecialOpcode::Dsra
                    | SpecialOpcode::Dsll32
                    | SpecialOpcode::Dsrl32
                    | SpecialOpcode::Dsra32
            )
        )
    }

    pub(super) fn is_high_shift(self) -> bool {
        matches!(
            self.function,
            Some(SpecialOpcode::Dsll32 | SpecialOpcode::Dsrl32 | SpecialOpcode::Dsra32)
        )
    }
}
