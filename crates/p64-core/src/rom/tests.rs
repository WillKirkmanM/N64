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
