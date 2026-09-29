#[derive(Default)]
pub struct IntegerRegisters {
    pub gpr: [u64; 32],
    pub hi: u64,
    pub lo: u64,
    pub llbit: bool,
}

impl IntegerRegisters {
    #[inline]
    pub fn gpr(&self, index: usize) -> u64 {
        self.gpr[index]
    }

    /// Retirement restores r0; retaining raw writes preserves hazards.
    #[inline]
    pub fn set_gpr(&mut self, index: usize, value: u64) {
        self.gpr[index] = value;
    }
}
