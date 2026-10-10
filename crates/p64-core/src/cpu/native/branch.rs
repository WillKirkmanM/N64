use super::{native_retirement_budget, retire_native_integer_instructions};
use crate::{recompiler::NativeBranchBlock, N64};

pub(super) struct ExecutedBranch {
    pub(super) taken: bool,
    pub(super) target: u64,
}

