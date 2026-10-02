#[derive(Copy, Clone, Default)]
pub struct NativePreparation {
    pub generation: u64,
    pub candidates: u8,
    pub prepared: u8,
}

impl NativePreparation {
    #[inline]
    pub(crate) fn is_candidate(&self, index: usize) -> bool {
        self.candidates & (1 << index) != 0
    }

    #[inline]
    pub(crate) fn was_prepared(&self, index: usize) -> bool {
        self.prepared & (1 << index) != 0
    }

    pub(crate) fn mark_prepared(&mut self, index: usize) {
        self.prepared |= 1 << index;
    }

    pub(crate) fn discard_candidate(&mut self, index: usize) {
        self.candidates &= !(1 << index);
    }

    pub(crate) fn reset(&mut self, generation: u64, candidates: u8) {
        self.generation = generation;
        self.candidates = candidates;
        self.prepared = 0;
    }
}
