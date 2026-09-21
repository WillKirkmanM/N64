//! AOT machine code is validated against the local emitter before execution.
use crate::recompiler::{compile_block, Engine, NativeCodeRecord, MAX_BLOCK_INSTRUCTIONS, MAX_BLOCK_RECORDS, MAX_SCALAR_RECORDS};
use sha2::{Digest, Sha256};
mod header;
mod reader;
mod run;
mod scanner;

use header::ArtifactHeader;
use reader::ArtifactReader;

const ARTIFACT_MAGIC: &[u8; 8] = b"P64AOT\0\0";
const ARTIFACT_VERSION: u32 = 5;
const X86_64_ARCHITECTURE_ID: u32 = 1;
pub const MAX_ARTIFACT_BYTES: usize = 128 * 1024 * 1024;
const MAX_SCALAR_ENTRIES: usize = MAX_SCALAR_RECORDS;
const MAX_BLOCK_ENTRIES: usize = MAX_BLOCK_RECORDS;
const MAX_ARTIFACT_ENTRIES: usize = MAX_SCALAR_ENTRIES + MAX_BLOCK_ENTRIES;
const MAX_NATIVE_CODE_BYTES: usize = 2048;
const SHA256_DIGEST_BYTES: usize = 32;
const ABI_WIN64: u32 = 1;
const ABI_SYSTEM_V: u32 = 2;
// Magic, version/architecture/ABI/record count, ROM length, and two hashes.
const ARTIFACT_HEADER_BYTES: usize = ARTIFACT_MAGIC.len()
    + 4 * std::mem::size_of::<u32>()
    + std::mem::size_of::<u64>()
    + 2 * SHA256_DIGEST_BYTES;
// Two record length words, at least one opcode, and at least one code byte.
const MIN_RECORD_BYTES: usize = 3 * std::mem::size_of::<u32>() + 1;

fn abi() -> Result<u32, String> {
    if !cfg!(target_arch = "x86_64") {
        return Err("AOT native artifacts require x86_64".into());
    }

    Ok(if cfg!(target_os = "windows") {
        ABI_WIN64
    } else {
        ABI_SYSTEM_V
    })
}

fn digest(data: &[u8]) -> [u8; SHA256_DIGEST_BYTES] {
    Sha256::digest(data).into()
}

pub fn normalize_rom(bytes: &[u8]) -> Result<Vec<u8>, String> {
    crate::rom::normalize_rom(bytes.to_vec()).map_err(|error| error.to_string())
}

fn check_canonical(rom: &[u8]) -> Result<(), String> {
    if !crate::rom::is_canonical(rom) {
        return Err("Expected a canonical big-endian N64 ROM".into());
    }

    Ok(())
}

