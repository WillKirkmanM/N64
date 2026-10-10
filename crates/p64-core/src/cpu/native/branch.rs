
use crate::{recompiler::NativeBranchBlock, N64};

pub(super) struct ExecutedBranch {
    pub(super) taken: bool,
    pub(super) target: u64,
}

