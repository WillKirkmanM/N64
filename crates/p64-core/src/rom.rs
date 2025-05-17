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

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

fn is_rom_path(path: &Path) -> bool {
    ["z64", "v64", "n64"]
        .iter()
        .any(|extension| has_extension(path, extension))
}

fn read_zip_rom(path: &Path) -> io::Result<Vec<u8>> {
    let mut archive = ZipArchive::new(fs::File::open(path)?)?;

    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;

        if entry.is_dir() || !entry.enclosed_name().is_some_and(|name| is_rom_path(&name)) {
            continue;
        }

        if entry.size() > MAX_ROM_BYTES as u64 {
            return Err(Error::new(ErrorKind::InvalidData, "ROM exceeds 128 MiB"));
        }

        let mut contents = Vec::new();
        entry
            .take(MAX_ROM_BYTES as u64 + 1)
            .read_to_end(&mut contents)?;

        return Ok(contents);
    }

    Err(Error::new(
        ErrorKind::InvalidData,
        "ZIP archive contains no N64 ROM",
    ))
}

pub fn try_read_rom(path: &Path) -> io::Result<Vec<u8>> {
    let contents = if has_extension(path, "zip") {
        read_zip_rom(path)?
    } else {
        let file = fs::File::open(path)?;
        if file.metadata()?.len() > MAX_ROM_BYTES as u64 {
            return Err(Error::new(ErrorKind::InvalidData, "ROM exceeds 128 MiB"));
        }
        let mut contents = Vec::new();
        file.take(MAX_ROM_BYTES as u64 + 1)
            .read_to_end(&mut contents)?;
        contents
    };

    normalize_rom(contents)
}

pub fn read_rom(path: &Path) -> Vec<u8> {
    try_read_rom(path)
        .unwrap_or_else(|error| panic!("Unable to read ROM {}: {error}", path.display()))
}

#[cfg(test)]
mod tests;
