use super::*;
use std::fs;
#[test]
fn complete_canonical_hash_and_name_match_all_three_byte_orders() {
    let directory = std::env::temp_dir().join(format!("p64-rom-metadata-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let mut canonical = vec![0; 128];
    canonical[..4].copy_from_slice(&[0x80, 0x37, 0x12, 0x40]);
    canonical[32..36].copy_from_slice(b"ABCD");
    canonical[52..56].copy_from_slice(b"TAIL");
    canonical[100] = 17;
    let expected = format!("{:x}", md5::compute(&canonical));
    for (extension, width) in [("z64", 1), ("v64", 2), ("n64", 4)] {
        let mut bytes = canonical.clone();
        for word in bytes.chunks_exact_mut(width) {
            word.reverse();
        }
        let path = directory.join(format!("game.{extension}"));
        fs::write(&path, bytes).unwrap();
        let info = RomInfo::new(&path).unwrap();
        assert_eq!(info.internal_name, "ABCD");
        assert_eq!(info.md5, expected);
    }
    let oversized = directory.join("oversized.z64");
    fs::File::create(&oversized)
        .unwrap()
        .set_len(p64_core::rom::MAX_ROM_BYTES as u64 + 1)
        .unwrap();
    assert!(RomInfo::new(&oversized).is_none());
    fs::remove_dir_all(directory).unwrap();
}
