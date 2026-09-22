use crate::recompiler::supported_memory_opcode;
use std::collections::BTreeSet;

const LOOKAHEAD_WORD: usize = 6;
const MIN_RUN_INSTRUCTIONS: usize = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RunKind {
    Integer,
    Memory,
}

pub(super) fn insert_run(
    blocks: &mut BTreeSet<Vec<u32>>,
    words: &[u32],
    start: usize,
    length: usize,
    kind: RunKind,
) {
    let eligible = |run: &[u32]| {
        kind == RunKind::Integer || run.iter().any(|opcode| supported_memory_opcode(*opcode))
    };

    if length < MIN_RUN_INSTRUCTIONS || !eligible(&words[start..start + length]) {
        return;
    }

    blocks.insert(words[start..start + length].to_vec());

    if start + MIN_RUN_INSTRUCTIONS <= LOOKAHEAD_WORD
        && start + length > LOOKAHEAD_WORD
        && eligible(&words[start..LOOKAHEAD_WORD])
    {
        blocks.insert(words[start..LOOKAHEAD_WORD].to_vec());
    }
}
