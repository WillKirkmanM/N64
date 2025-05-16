use std::{
    fs,
    io::{self, Error, ErrorKind, Read},
    path::Path,
};

use zip::ZipArchive;

pub const MAX_ROM_BYTES: usize = 128 * 1024 * 1024;

const ROM_HEADER_BYTES: usize = 64;

const ROM_WORD_BYTES: usize = std::mem::size_of::<u32>();

const ROM_BYTE_PAIR_BYTES: usize = std::mem::size_of::<u16>();

const Z64_MAGIC: [u8; 4] = [0x80, 0x37, 0x12, 0x40];

const V64_MAGIC: [u8; 4] = [0x37, 0x80, 0x40, 0x12];

const N64_MAGIC: [u8; 4] = [0x40, 0x12, 0x37, 0x80];

fn valid_size(contents: &[u8]) -> bool {
    (ROM_HEADER_BYTES..=MAX_ROM_BYTES).contains(&contents.len())
        && contents.len().is_multiple_of(ROM_WORD_BYTES)
}

pub(crate) fn is_canonical(contents: &[u8]) -> bool {
    valid_size(contents) && contents.starts_with(&Z64_MAGIC)
}

/// Convert V64 byte pairs and N64 words to canonical big-endian Z64.
pub fn normalize_rom(mut contents: Vec<u8>) -> io::Result<Vec<u8>> {
    if !valid_size(&contents) {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "ROM must contain a 64-byte header, be word aligned, and be at most 128 MiB",
        ));
    }

    let magic = *contents
        .first_chunk::<ROM_WORD_BYTES>()
        .expect("ROM header length validated");

    match magic {
        Z64_MAGIC => {}
        V64_MAGIC => {
            for pair in contents.chunks_exact_mut(ROM_BYTE_PAIR_BYTES) {
                pair.swap(0, 1);
            }
        }
        N64_MAGIC => {
            for word in contents.chunks_exact_mut(ROM_WORD_BYTES) {
                word.reverse();
            }
        }
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Unknown ROM byte order (expected z64, v64, or n64)",
            ));
        }
    }

    Ok(contents)
}

pub fn read_rom(file_path: &Path) -> Vec<u8> {
    let mut contents = vec![];
    if file_path.extension().unwrap().to_ascii_lowercase() == "zip" {
        let zip_file = fs::File::open(file_path).unwrap();
        let mut archive = ZipArchive::new(zip_file).unwrap();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let extension = file
                .enclosed_name()
                .unwrap()
                .extension()
                .unwrap()
                .to_ascii_lowercase();
            if extension == "z64" || extension == "n64" || extension == "v64" {
                file.read_to_end(&mut contents)
                    .expect("This file is not an accepted N64 ROM format");
                break;
            }
        }
    } else {
        contents = fs::read(file_path).expect("Unable to read the ROM file");
    }

    normalize_rom(contents)
        .expect("Could not read the ROM format? is it a z64, n64, or v64?")
}
