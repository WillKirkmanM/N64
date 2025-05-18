use super::*;
use std::io::{Cursor, Write};
use zip::{write::SimpleFileOptions, ZipWriter};

fn canonical_rom() -> Vec<u8> {
    let mut rom: Vec<u8> = (0..128).collect();
    rom[..4].copy_from_slice(&Z64_MAGIC);

    rom
}

#[test]
fn every_rom_order_preserves_every_byte_after_normalization() {
    let expected = canonical_rom();

    for width in [1, 2, 4] {
        let mut input = expected.clone();

        for chunk in input.chunks_exact_mut(width) {
            chunk.reverse();
        }

        assert_eq!(normalize_rom(input).unwrap(), expected);
    }
}

#[test]
fn malformed_headers_and_trailing_bytes_are_errors() {
    for contents in [
        vec![],
        vec![0; 3],
        vec![0; 64],
        canonical_rom()[..127].to_vec(),
    ] {
        assert_eq!(
            normalize_rom(contents).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }
}

#[test]
fn zip_reader_skips_directories_and_extensionless_entries() {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    zip.add_directory("folder/", options).unwrap();
    zip.start_file("README", options).unwrap();
    zip.write_all(b"notes").unwrap();
    zip.start_file("folder/TEST.Z64", options).unwrap();
    let expected = canonical_rom();
    zip.write_all(&expected).unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    let path = std::env::temp_dir().join(format!("p64-rom-test-{}.zip", std::process::id()));
    fs::write(&path, bytes).unwrap();
    let result = try_read_rom(&path);
    fs::remove_file(&path).unwrap();

    assert_eq!(result.unwrap(), expected);
}
