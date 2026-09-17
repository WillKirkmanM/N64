#[derive(Clone, Copy, Debug)]
pub(in crate::recompiler) struct BlockMetadata {
    pub(in crate::recompiler) instruction_count: usize,
    pub(in crate::recompiler) first_opcode: u32,
    pub(in crate::recompiler) wide: bool,
}

impl BlockMetadata {}
