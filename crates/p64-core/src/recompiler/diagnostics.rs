use super::super::super::MAX_BLOCK_INSTRUCTIONS;

const PRIMARY_OPCODE_COUNT: usize = 64;
const NATIVE_REJECT_COUNT: usize = crate::recompiler::NativeReject::ALL.len();

pub struct ExecutionDiagnostics {
    pub native_block_length_counts: [u64; MAX_BLOCK_INSTRUCTIONS + 1],
    pub native_branch_length_counts: [u64; MAX_BLOCK_INSTRUCTIONS + 1],
    pub scalar_opcode_counts: [u64; PRIMARY_OPCODE_COUNT],
    pub native_reject_counts: [u64; NATIVE_REJECT_COUNT],
}

impl ExecutionDiagnostics {
    pub(super) fn extend_branch(&mut self, branch_length: usize) {
        for histogram in [
            &mut self.native_block_length_counts,
            &mut self.native_branch_length_counts,
        ] {
            histogram[branch_length] -= 1;
            histogram[branch_length + 1] += 1;
        }
    }
}

impl Default for ExecutionDiagnostics {
    fn default() -> Self {
        Self {
            native_block_length_counts: [0; MAX_BLOCK_INSTRUCTIONS + 1],
            native_branch_length_counts: [0; MAX_BLOCK_INSTRUCTIONS + 1],
            scalar_opcode_counts: [0; PRIMARY_OPCODE_COUNT],
            native_reject_counts: [0; NATIVE_REJECT_COUNT],
        }
    }
}
