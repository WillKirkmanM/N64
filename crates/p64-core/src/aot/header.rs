use super::{abi, digest, ArtifactReader, ARTIFACT_MAGIC, ARTIFACT_VERSION, SHA256_DIGEST_BYTES, X86_64_ARCHITECTURE_ID};

pub(super) struct ArtifactHeader {
    target_abi: u32,
    rom_length: u64,
    rom_digest: [u8; SHA256_DIGEST_BYTES],
    pub(super) record_count: u32,
    payload_digest: [u8; SHA256_DIGEST_BYTES],
}

impl ArtifactHeader {

}
