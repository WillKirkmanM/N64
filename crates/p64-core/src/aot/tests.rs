use super::*;
use crate::recompiler::{supported_branch_opcode, supported_memory_opcode};
fn rom_with_words(words: &[u32]) -> Vec<u8> {
    let mut rom = vec![0; 64];
    rom[..4].copy_from_slice(&[0x80, 0x37, 0x12, 0x40]);
    for opcode in words {
        rom.extend_from_slice(&opcode.to_be_bytes());
    }
    rom
}

fn rom() -> Vec<u8> {
    // ADDIU r2,r1,-1, ORI r3,r2,0x1234; terminal J leaves its delay scalar.
    rom_with_words(&[0x2422ffff, 0x34431234, 0x08000000])
}

fn rehash_payload(bytes: &mut [u8]) {
    let hash = digest(&bytes[ARTIFACT_HEADER_BYTES..]);
    bytes[64..96].copy_from_slice(&hash);
}

#[test]
fn deterministic_roundtrip_and_byte_orders() {
    let canonical = rom();
    let artifact = compile_artifact(&canonical).unwrap();
    assert_eq!(artifact, compile_artifact(&canonical).unwrap());
    let codes = decode_records(&artifact, &canonical).unwrap();
    assert!(codes.iter().any(|(ops, _)| ops.as_slice() == [0x2422ffff]));
    assert!(codes
        .iter()
        .any(|(ops, _)| ops.as_slice() == [0x2422ffff, 0x34431234]));
    assert!(codes
        .iter()
        .any(|(ops, _)| ops.as_slice() == [0x2422ffff, 0x34431234, 0x08000000]));
    assert!(!codes.iter().any(|(ops, _)| ops.as_slice() == [0x08000000]));
    assert!(load_artifact(&artifact, &canonical).is_ok());
    let mut v64 = canonical.clone();
    for pair in v64.chunks_exact_mut(2) {
        pair.swap(0, 1);
    }
    let mut n64 = canonical.clone();
    for word in n64.chunks_exact_mut(4) {
        word.reverse();
    }
    assert_eq!(normalize_rom(&v64).unwrap(), canonical);
    assert_eq!(normalize_rom(&n64).unwrap(), canonical);
}
#[test]
fn rejects_wrong_rom_corruption_and_forged_checksum() {
    let canonical = rom();
    let original = compile_artifact(&canonical).unwrap();
    let mut other_rom = canonical.clone();
    other_rom[40] ^= 1;
    assert!(load_artifact(&original, &other_rom).is_err());
    for offset in [0, 8, 12, 16, 20, 28, 60, 64, original.len() - 1] {
        let mut changed = original.clone();
        changed[offset] ^= 0xff;
        assert!(
            load_artifact(&changed, &canonical).is_err(),
            "offset {offset}"
        );
    }
    let mut forged = original.clone();
    *forged.last_mut().unwrap() ^= 0xff;
    rehash_payload(&mut forged);
    assert!(load_artifact(&forged, &canonical)
        .err()
        .unwrap()
        .contains("trusted emitter"));
    for end in 0..original.len() {
        assert!(load_artifact(&original[..end], &canonical).is_err());
    }
    let mut trailing = original.clone();
    trailing.push(0);
    rehash_payload(&mut trailing);
    assert!(load_artifact(&trailing, &canonical)
        .err()
        .unwrap()
        .contains("Trailing"));
    assert!(normalize_rom(&[]).is_err());
    let mut old_version = original.clone();
    old_version[8..12].copy_from_slice(&1u32.to_le_bytes());
    assert!(load_artifact(&old_version, &canonical)
        .err()
        .unwrap()
        .contains("rebuild"));
}
#[test]
fn persists_runtime_runs_and_cold_lookahead_prefixes_without_crossing_lines() {
    let operations: Vec<u32> = (1..=8).map(|imm| 0x24220000 | imm).collect();
    let mut canonical = rom_with_words(&operations);
    canonical.extend_from_slice(&0x24220009u32.to_be_bytes());
    canonical.extend_from_slice(&0x08000000u32.to_be_bytes());
    canonical.extend_from_slice(&0x2422000au32.to_be_bytes());
    let codes = decode_records(&compile_artifact(&canonical).unwrap(), &canonical).unwrap();
    for start in 0..8 {
        for len in 2..=8 - start {
            let requested = len == 8 - start || (start + len == 6 && start + 2 <= 6);
            assert_eq!(
                codes
                    .iter()
                    .any(|(ops, _)| ops.as_slice() == &operations[start..start + len]),
                requested
            );
        }
    }
    assert!(codes
        .iter()
        .any(|(ops, _)| ops.as_slice() == [0x24220009, 0x08000000]));
    assert!(codes.iter().all(|(ops, _)| !ops[..ops.len() - 1]
        .iter()
        .any(|op| supported_branch_opcode(*op))));
    assert!(!codes
        .iter()
        .any(|(ops, _)| ops.windows(2).any(|pair| pair == [0x24220008, 0x24220009])));
    assert!(!codes
        .iter()
        .any(|(ops, _)| ops.as_slice() == [0x24220009, 0x2422000a]));
}
#[test]
fn compact_memory_variants_cover_cold_lookahead_and_two_instruction_runs() {
    let operations = [
        0x24220001u32,
        0xDC230000,
        0x24630001,
        0xFC230008,
        0x24420002,
        0x24420003,
        0xDC240008,
        0xFC240000,
    ];
    let mut canonical = rom_with_words(&operations);
    let short = [0xDC250000u32, 0xFC250008];
    for opcode in short {
        canonical.extend_from_slice(&opcode.to_be_bytes());
    }
    canonical.extend_from_slice(&0x08000000u32.to_be_bytes());
    let records = decode_records(&compile_artifact(&canonical).unwrap(), &canonical).unwrap();
    for start in 0..7 {
        assert!(records
            .iter()
            .any(|(ops, _)| ops.as_slice() == &operations[start..]));
        if start + 2 <= 6 {
            assert!(records
                .iter()
                .any(|(ops, _)| ops.as_slice() == &operations[start..6]));
        }
    }
    assert!(records.iter().any(|(ops, _)| ops.as_slice() == short));
    assert!(!records
        .iter()
        .any(|(ops, _)| ops.as_slice() == &operations[..3]));
    assert!(!records
        .iter()
        .any(|(ops, _)| ops.as_slice() == &operations[1..4]));
}

#[test]
fn loaded_native_alu_matches_scalar_oracle_and_jit() {
    let canonical = rom();
    let artifact = compile_artifact(&canonical).unwrap();
    let mut aot = load_artifact(&artifact, &canonical).unwrap();
    let mut jit = Engine::jit();
    let compiled = aot.compiled_instructions();
    for value in [
        0,
        1,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_ffff,
        0x1234_5678_9abc_def0,
        u64::MAX,
    ] {
        let mut expected = [0; 32];
        expected[1] = value;
        let mut actual = expected;
        let mut dynamic = expected;
        for opcode in [0x2422ffff, 0x34431234] {
            match opcode {
                0x2422ffff => expected[2] = (value as u32).wrapping_sub(1) as i32 as i64 as u64,
                0x34431234 => expected[3] = expected[2] | 0x1234,
                _ => unreachable!(),
            }
            assert!(aot.try_execute(opcode, &mut actual));
            assert!(jit.try_execute(opcode, &mut dynamic));
            assert_eq!(actual, expected);
            assert_eq!(dynamic, expected);
        }
        let before = actual;
        assert!(!aot.try_execute(0x08000000, &mut actual));
        assert_eq!(actual, before);
        assert_eq!(aot.compiled_instructions(), compiled);
    }
    assert_eq!(aot.stats.native_executions, 14);
    assert_eq!(aot.stats.fallback_executions, 7);
}
