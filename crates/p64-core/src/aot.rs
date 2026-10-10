//! AOT machine code is validated against the local emitter before execution.
use crate::recompiler::{compile_block, Engine, NativeCodeRecord, MAX_BLOCK_INSTRUCTIONS, MAX_BLOCK_RECORDS, MAX_SCALAR_RECORDS};
use sha2::{Digest, Sha256};
mod header;
mod reader;
mod run;
mod scanner;

use header::ArtifactHeader;
use reader::ArtifactReader;

#[cfg(all(test, target_arch = "x86_64"))]
mod tests;

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

/// Requires a ROM in canonical big-endian byte order.
pub fn compile_artifact(rom: &[u8]) -> Result<Vec<u8>, String> {
    check_canonical(rom)?;
    abi()?;
    let blocks = scanner::scan_rom(rom)?;
    let mut out = vec![0; ARTIFACT_HEADER_BYTES];
    let mut count = 0u32;

    for block in blocks {
        if let Some(code) = compile_block(&block) {
            if code.is_empty()
                || code.len() > MAX_NATIVE_CODE_BYTES
                || count as usize >= MAX_ARTIFACT_ENTRIES
            {
                return Err("AOT emitter or entry count exceeds artifact limits".into());
            }

            out.extend_from_slice(&(block.len() as u32).to_le_bytes());
            out.extend_from_slice(&(code.len() as u32).to_le_bytes());
            for opcode in block {
                out.extend_from_slice(&opcode.to_le_bytes());
            }

            out.extend_from_slice(&code);
            count += 1;

            if out.len() > MAX_ARTIFACT_BYTES {
                return Err("AOT artifact exceeds 128 MiB".into());
            }
        }
    }

    let mut header = Vec::with_capacity(ARTIFACT_HEADER_BYTES);
    ArtifactHeader::new(rom, &out[ARTIFACT_HEADER_BYTES..], count)?.write(&mut header);
    out[..ARTIFACT_HEADER_BYTES].copy_from_slice(&header);

    Ok(out)
}

/// Checks the header and returns its record count.
pub fn artifact_record_count(bytes: &[u8]) -> Result<u32, String> {
    Ok(ArtifactHeader::read(&mut ArtifactReader::new(bytes))?.record_count)
}

/// Validate all persisted machine code before passing any bytes to executable memory.
pub fn load_artifact(bytes: &[u8], rom: &[u8]) -> Result<Engine, String> {
    let codes = decode_records(bytes, rom)?;

    Engine::from_aot_blocks(codes)
}

fn decode_records(bytes: &[u8], rom: &[u8]) -> Result<Vec<NativeCodeRecord>, String> {
    check_canonical(rom)?;
    if bytes.len() < ARTIFACT_HEADER_BYTES
        || bytes.len() > MAX_ARTIFACT_BYTES
        || &bytes[..ARTIFACT_MAGIC.len()] != ARTIFACT_MAGIC
    {
        return Err("Invalid AOT artifact magic or size".into());
    }

    let mut reader = ArtifactReader::new(bytes);
    let header = ArtifactHeader::read(&mut reader)?;
    header.validate_rom(rom)?;
    let count = header.record_count as usize;

    if count > MAX_ARTIFACT_ENTRIES
        || count > (bytes.len() - ARTIFACT_HEADER_BYTES) / MIN_RECORD_BYTES
    {
        return Err("Invalid AOT entry count".into());
    }

    header.validate_payload(&bytes[ARTIFACT_HEADER_BYTES..])?;
    let mut result = Vec::with_capacity(count);
    let mut previous: Option<Vec<u32>> = None;
    let mut scalars = 0;
    let mut blocks = 0;

    for _ in 0..count {
        let instruction_count = reader.u32()? as usize;
        let len = reader.u32()? as usize;

        if instruction_count == 0
            || instruction_count > MAX_BLOCK_INSTRUCTIONS
            || len == 0
            || len > MAX_NATIVE_CODE_BYTES
        {
            return Err("Invalid AOT code size or block instruction count".into());
        }

        let mut opcodes = Vec::with_capacity(instruction_count);

        for _ in 0..instruction_count {
            opcodes.push(reader.u32()?);
        }

        if previous.as_ref().is_some_and(|last| last >= &opcodes) {
            return Err("Invalid AOT opcode ordering".into());
        }

        if instruction_count == 1 {
            scalars += 1;
        } else {
            blocks += 1;
        }

        if scalars > MAX_SCALAR_ENTRIES || blocks > MAX_BLOCK_ENTRIES {
            return Err("AOT scalar or block entry limit exceeded".into());
        }

        let code = reader.take(len).map_err(|_| "Truncated AOT machine code")?;

        previous = Some(opcodes.clone());
        result.push((opcodes, code.to_vec()));
    }

    if !reader.is_empty() {
        return Err("Trailing AOT artifact data".into());
    }

    Ok(result)
}
