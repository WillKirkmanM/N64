use super::{
    abi, digest, ArtifactReader, ARTIFACT_MAGIC, ARTIFACT_VERSION, SHA256_DIGEST_BYTES,
    X86_64_ARCHITECTURE_ID,
};

pub(super) struct ArtifactHeader {
    target_abi: u32,
    rom_length: u64,
    rom_digest: [u8; SHA256_DIGEST_BYTES],
    pub(super) record_count: u32,
    payload_digest: [u8; SHA256_DIGEST_BYTES],
}

impl ArtifactHeader {
    pub(super) fn new(rom: &[u8], payload: &[u8], record_count: u32) -> Result<Self, String> {
        Ok(Self {
            target_abi: abi()?,
            rom_length: rom.len() as u64,
            rom_digest: digest(rom),
            record_count,
            payload_digest: digest(payload),
        })
    }

    pub(super) fn read(reader: &mut ArtifactReader<'_>) -> Result<Self, String> {
        if reader.array::<{ ARTIFACT_MAGIC.len() }>()? != *ARTIFACT_MAGIC {
            return Err("Invalid AOT artifact magic or size".into());
        }

        if reader.u32()? != ARTIFACT_VERSION {
            return Err(
                "Unsupported AOT artifact version; rebuild with the current n64-aot compiler"
                    .into(),
            );
        }

        if reader.u32()? != X86_64_ARCHITECTURE_ID {
            return Err("Unsupported AOT version, architecture, or ABI".into());
        }

        let target_abi = reader.u32()?;

        if target_abi != abi()? {
            return Err("Unsupported AOT version, architecture, or ABI".into());
        }

        Ok(Self {
            target_abi,
            rom_length: reader.u64()?,
            rom_digest: reader.array()?,
            record_count: reader.u32()?,
            payload_digest: reader.array()?,
        })
    }

    pub(super) fn validate_rom(&self, rom: &[u8]) -> Result<(), String> {
        if self.rom_length != rom.len() as u64 || self.rom_digest != digest(rom) {
            return Err("AOT artifact belongs to a different ROM".into());
        }

        Ok(())
    }

    pub(super) fn validate_payload(&self, payload: &[u8]) -> Result<(), String> {
        if self.payload_digest != digest(payload) {
            return Err("AOT payload checksum mismatch".into());
        }

        Ok(())
    }
}
