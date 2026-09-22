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
