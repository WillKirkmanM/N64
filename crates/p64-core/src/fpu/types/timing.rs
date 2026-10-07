#[derive(Default)]
pub struct FpuTiming {
    pub pending_write: Option<u8>,
    pub previous_write: Option<u8>,
}

impl FpuTiming {
    pub(crate) fn depends_on(&self, source: usize, target: usize) -> bool {
        self.previous_write.is_some_and(|register| {
            usize::from(register) == source || usize::from(register) == target
        })
    }

    pub fn begin_instruction(&mut self) {
        self.previous_write = self.pending_write.take();
    }
}
