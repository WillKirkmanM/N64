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

impl Operands {}
