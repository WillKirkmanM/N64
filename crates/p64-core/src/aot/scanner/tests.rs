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
