use super::*;

fn bytes(words: &[u32]) -> Vec<u8> {
    words
        .iter()
        .flat_map(|opcode| opcode.to_be_bytes())
        .collect()
}

fn fixture() -> Vec<u8> {
    bytes(&[
        0x24220001, 0x24220002, 0x24220003, 0x24220004, 0x24220005, 0x24220006, 0x24220007,
        0x24220008, 0x8c220000, 0x24220009, 0xac220004, 0x2422000a, 0x08000000, 0x2422000b,
        0x2422000c, 0x2422000d,
    ])
}

#[test]
fn selection_is_bounded_deterministic_and_prioritizes_early_rom_words() {
    let rom = fixture();
    for scalar_limit in 0..=16 {
        for block_limit in 0..=16 {
            let selected = scan_with_limits(&rom, scalar_limit, block_limit);
            assert_eq!(selected, scan_with_limits(&rom, scalar_limit, block_limit));
            assert!(selected.iter().filter(|ops| ops.len() == 1).count() <= scalar_limit);
            assert!(selected.iter().filter(|ops| ops.len() > 1).count() <= block_limit);
        }
    }
    let selected = scan_with_limits(&rom, 3, 1);
    let scalars: Vec<_> = selected
        .iter()
        .filter(|ops| ops.len() == 1)
        .cloned()
        .collect();
    assert_eq!(
        scalars,
        [vec![0x24220001], vec![0x24220002], vec![0x24220003]]
    );
    let blocks: Vec<_> = selected.iter().filter(|ops| ops.len() > 1).collect();
    assert_eq!(blocks.len(), 1);
    assert_eq!(
        blocks[0],
        &vec![0x24220001, 0x24220002, 0x24220003, 0x24220004, 0x24220005, 0x24220006]
    );
    let mut extended = rom.clone();
    extended.extend(bytes(&[0x2422fffe; 64]));
    assert_eq!(selected, scan_with_limits(&extended, 3, 1));
}

#[test]
fn bounded_insertion_handles_duplicates_and_multiple_run_candidates_at_capacity() {
    let mut selected = BTreeSet::from([vec![1, 2]]);
    insert_bounded(&mut selected, [vec![1, 2], vec![2, 3], vec![3, 4]], 2);
    assert_eq!(selected, BTreeSet::from([vec![1, 2], vec![2, 3]]));
    insert_bounded(&mut selected, [vec![0, 1]], 2);
    assert_eq!(selected.len(), 2);
    let selected = scan_with_limits(&fixture(), 0, 2);
    assert_eq!(selected.len(), 2);
    assert!(selected.iter().all(|ops| ops.len() > 1));
}

#[test]
fn below_capacity_selection_preserves_existing_records() {
    let rom = fixture();
    let mut expected = BTreeSet::new();
    for line in rom.chunks(CACHE_LINE_BYTES) {
        let words: Vec<u32> = line
            .chunks_exact(INSTRUCTION_BYTES)
            .map(|word| u32::from_be_bytes(word.try_into().unwrap()))
            .collect();
        for &opcode in &words {
            if supported_opcode(opcode) {
                expected.insert(vec![opcode]);
            }
        }
        let mut blocks = BTreeSet::new();
        for start in 0..words.len().saturating_sub(1) {
            let length = words[start..]
                .iter()
                .take_while(|&&op| supported_opcode(op))
                .count();
            insert_run(&mut blocks, &words, start, length, RunKind::Integer);
            if length >= 1
                && start + length < words.len()
                && supported_branch_opcode(words[start + length])
            {
                blocks.insert(words[start..=start + length].to_vec());
            }
            let length = words[start..]
                .iter()
                .take_while(|&&op| supported_opcode(op) || supported_memory_opcode(op))
                .count();
            insert_run(&mut blocks, &words, start, length, RunKind::Memory);
        }
        expected.extend(blocks);
    }
    assert_eq!(scan_with_limits(&rom, 100, 100), expected);
}

#[cfg(target_arch = "x86_64")]
#[test]
fn unselected_native_code_remains_a_frozen_interpreter_fallback() {
    use crate::recompiler::{compile_block, Engine};
    let selected = scan_with_limits(&fixture(), 2, 1);
    let records = selected
        .into_iter()
        .map(|ops| {
            let code = compile_block(&ops).unwrap();
            (ops, code)
        })
        .collect();
    let mut engine = Engine::from_aot_blocks(records).unwrap();
    let counts = (engine.compiled_instructions(), engine.compiled_blocks());
    let mut registers = [0u64; 32];
    assert!(engine.try_execute(0x24220001, &mut registers));
    assert_eq!(registers[2], 1);
    let before = registers;
    assert!(!engine.try_execute(0x2422000d, &mut registers));
    assert!(engine.prepare_block(&[0x2422000b, 0x2422000c]).is_none());
    assert_eq!(registers, before);
    assert_eq!(
        (engine.compiled_instructions(), engine.compiled_blocks()),
        counts
    );
}
