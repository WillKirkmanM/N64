use crate::memory::AccessType;

pub(super) const PAGE_SHIFT: u32 = 12;
pub(super) const PAGE_BYTES: u64 = 1 << PAGE_SHIFT;
const UNCACHED: u8 = 2;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct TlbPage {
    pub pfn: u64,
    pub coherency: u8,
    pub dirty: bool,
    pub valid: bool,
}

impl TlbPage {
    #[inline]
    pub fn cacheable(self) -> bool {
        self.coherency != UNCACHED
    }

    #[inline]
    pub fn permits(self, access: AccessType) -> bool {
        self.valid && (access == AccessType::Read || self.dirty)
    }

    #[inline]
    pub fn translate(self, address: u64, page_size: u64) -> u64 {
        let offset_mask = page_size - 1;

        (self.physical_base() & !offset_mask) | (address & offset_mask)
    }

    pub(super) fn physical_base(self) -> u64 {
        self.pfn << PAGE_SHIFT
    }
}
