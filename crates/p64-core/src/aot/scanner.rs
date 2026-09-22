use super::{
    run::{insert_run, RunKind},
    MAX_BLOCK_ENTRIES, MAX_SCALAR_ENTRIES,
};
use crate::recompiler::{supported_branch_opcode, supported_memory_opcode, supported_opcode};
use std::collections::BTreeSet;

const INSTRUCTION_BYTES: usize = 4;
const CACHE_LINE_BYTES: usize = 32;
pub(super) fn scan_rom(rom: &[u8]) -> Result<BTreeSet<Vec<u32>>, String> {
    Ok(scan_with_limits(rom, MAX_SCALAR_ENTRIES, MAX_BLOCK_ENTRIES))
}

fn insert_bounded<T: Ord>(
    selected: &mut BTreeSet<T>,
    candidates: impl IntoIterator<Item = T>,
    limit: usize,
) {
    for candidate in candidates {
        if selected.len() == limit {
            break;
        }
        selected.insert(candidate);
    }
}

fn scan_with_limits(rom: &[u8], scalar_limit: usize, block_limit: usize) -> BTreeSet<Vec<u32>> {
    let mut opcodes = BTreeSet::new();

    for word in rom.chunks_exact(INSTRUCTION_BYTES) {
        let opcode = u32::from_be_bytes(word.try_into().unwrap());

        if opcodes.len() == scalar_limit {
            break;
        }
        if supported_opcode(opcode) {
            insert_bounded(&mut opcodes, [opcode], scalar_limit);
        }
    }

    let mut blocks = BTreeSet::new();

    'lines: for line in rom.chunks(CACHE_LINE_BYTES) {
        if blocks.len() == block_limit {
            break;
        }
        let words: Vec<u32> = line
            .chunks_exact(INSTRUCTION_BYTES)
            .map(|word| u32::from_be_bytes(word.try_into().unwrap()))
            .collect();

        for start in 0..words.len().saturating_sub(1) {
            let mut candidates = BTreeSet::new();
            let length = words[start..]
                .iter()
                .take_while(|&&opcode| supported_opcode(opcode))
                .count();
            insert_run(&mut candidates, &words, start, length, RunKind::Integer);
            if length >= 1
                && start + length < words.len()
                && supported_branch_opcode(words[start + length])
            {
                candidates.insert(words[start..=start + length].to_vec());
            }

            let mixed_length = words[start..]
                .iter()
                .take_while(|&&opcode| supported_opcode(opcode) || supported_memory_opcode(opcode))
                .count();
            insert_run(
                &mut candidates,
                &words,
                start,
                mixed_length,
                RunKind::Memory,
            );

            insert_bounded(&mut blocks, candidates, block_limit);
            if blocks.len() == block_limit {
                break 'lines;
            }
        }
    }

    blocks.extend(opcodes.into_iter().map(|opcode| vec![opcode]));

    blocks
}
