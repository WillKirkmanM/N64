pub(super) struct ArtifactReader<'a> {
    remaining: &'a [u8],
}

impl<'a> ArtifactReader<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }

    pub(super) fn take(&mut self, length: usize) -> Result<&'a [u8], String> {
        let (value, remaining) = self
            .remaining
            .split_at_checked(length)
            .ok_or("Truncated AOT artifact")?;
        self.remaining = remaining;

        Ok(value)
    }

    pub(super) fn array<const N: usize>(&mut self) -> Result<[u8; N], String> {
        self.take(N)?
            .try_into()
            .map_err(|_| "Truncated AOT artifact".into())
    }

    pub(super) fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub(super) fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    pub(super) fn is_empty(&self) -> bool {
        self.remaining.is_empty()
    }
}
