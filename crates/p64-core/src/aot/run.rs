use crate::recompiler::supported_memory_opcode;
use std::collections::BTreeSet;

const LOOKAHEAD_WORD: usize = 6;
const MIN_RUN_INSTRUCTIONS: usize = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RunKind {
    Integer,
    Memory,
}
