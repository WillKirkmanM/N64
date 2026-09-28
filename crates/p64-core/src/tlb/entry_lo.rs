use super::TlbPage;

const PFN_SHIFT: u32 = 6;
const PFN_MASK: u64 = 0xF_FFFF;
const COHERENCY_SHIFT: u32 = 3;
const COHERENCY_MASK: u64 = 7;
const DIRTY: u64 = 1 << 2;
const VALID: u64 = 1 << 1;
const GLOBAL: u64 = 1;

#[derive(Copy, Clone)]
pub(super) struct EntryLo(pub(super) u64);

impl EntryLo {
    pub(super) fn page(self) -> TlbPage {
        TlbPage {
            pfn: (self.0 >> PFN_SHIFT) & PFN_MASK,
            coherency: ((self.0 >> COHERENCY_SHIFT) & COHERENCY_MASK) as u8,
            dirty: self.0 & DIRTY != 0,
            valid: self.0 & VALID != 0,
        }
    }

    pub(super) fn global(self) -> bool {
        self.0 & GLOBAL != 0
    }

    pub(super) fn encode(page: TlbPage, global: bool) -> Self {
        Self(
            (page.pfn << PFN_SHIFT)
                | (u64::from(page.coherency) << COHERENCY_SHIFT)
                | (u64::from(page.dirty) * DIRTY)
                | (u64::from(page.valid) * VALID)
                | u64::from(global),
        )
    }
}
