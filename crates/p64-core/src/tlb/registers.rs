use super::entry_lo::EntryLo;
use crate::mmu::{Mmu, MmuRegister};

/// Captures the four CP0 inputs before a TLB operation mutates its tables.
#[derive(Copy, Clone)]
pub(super) struct RegisterSnapshot {
    pub(super) mask: u64,
    pub(super) hi: u64,
    pub(super) lo: [EntryLo; 2],
}

impl RegisterSnapshot {
    pub(super) fn read(cp0: &Mmu) -> Self {
        Self {
            mask: cp0.register(MmuRegister::PageMask),
            hi: cp0.register(MmuRegister::EntryHi),
            lo: [
                EntryLo(cp0.register(MmuRegister::EntryLo0)),
                EntryLo(cp0.register(MmuRegister::EntryLo1)),
            ],
        }
    }

    pub(super) fn write(self, cp0: &mut Mmu) {
        for (register, value) in [
            (MmuRegister::PageMask, self.mask),
            (MmuRegister::EntryHi, self.hi),
            (MmuRegister::EntryLo0, self.lo[0].0),
            (MmuRegister::EntryLo1, self.lo[1].0),
        ] {
            cp0.set_register(register, value);
        }
    }
}
