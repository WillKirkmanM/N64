#[derive(Clone, Copy, Debug)]
pub(in crate::recompiler) struct BlockMetadata {
    pub(in crate::recompiler) instruction_count: usize,
    pub(in crate::recompiler) first_opcode: u32,
    pub(in crate::recompiler) wide: bool,
}

impl BlockMetadata {
    pub(in crate::recompiler) fn new(opcodes: &[u32]) -> Self {
        Self {
            instruction_count: opcodes.len(),
            first_opcode: opcodes[0],
            wide: opcodes
                .iter()
                .any(|opcode| super::super::is_wide_opcode(*opcode)),
        }
    }
}
