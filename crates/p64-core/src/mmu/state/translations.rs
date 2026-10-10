use crate::tlb::{self, TlbEntry};

pub struct MmuTranslations {
    pub tlb_entries: [TlbEntry; tlb::ENTRY_COUNT],
}

impl Default for MmuTranslations {
    fn default() -> Self {
        Self {
            tlb_entries: [TlbEntry::default(); tlb::ENTRY_COUNT],
        }
    }
}
