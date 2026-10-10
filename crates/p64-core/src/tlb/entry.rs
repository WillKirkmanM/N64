use super::{entry_hi::{EntryHi, COMPATIBLE_VPN_MASK, EXTENDED_VPN_MASK, VPN_SHIFT}, entry_lo::EntryLo, page::PAGE_BYTES, registers::RegisterSnapshot, TlbPage};

const PAGE_MASK_PAIRS: u64 = 0b1010_1010_1010;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct TlbEntry {
    pub mask: u64,
    pub hi: EntryHi,
    pub global: bool,
    pub pages: [TlbPage; 2],
}

impl TlbEntry {
    pub(super) fn from_registers(registers: RegisterSnapshot) -> Self {
        // The VR4300 accepts only paired page-mask bits, then mirrors each odd bit.
        let pairs = (registers.mask >> VPN_SHIFT) & PAGE_MASK_PAIRS;
        let mask = pairs | (pairs >> 1);
        let mut hi = EntryHi::decode(registers.hi);
        hi.vpn2 &= !mask;

        Self {
            mask,
            hi,
            global: registers.lo.iter().all(|lo| lo.global()),
            pages: [registers.lo[0].page(), registers.lo[1].page()],
        }
    }

    pub(super) fn registers(self) -> RegisterSnapshot {
        RegisterSnapshot {
            mask: self.mask << VPN_SHIFT,
            hi: self.hi.encode(),
            lo: self.pages.map(|page| EntryLo::encode(page, self.global)),
        }
    }

    #[inline]
    pub fn page(self, odd: bool) -> TlbPage {
        self.pages[usize::from(odd)]
    }

    #[inline]
    pub fn page_size(self) -> u64 {
        (self.mask + 1) * PAGE_BYTES
    }

    pub(super) fn matches(self, query: EntryHi, extended: bool) -> bool {
        let vpn_mask = if extended {
            EXTENDED_VPN_MASK
        } else {
            COMPATIBLE_VPN_MASK
        };

        (!extended || self.hi.region == query.region)
            && (self.global || self.hi.asid == query.asid)
            && (self.hi.vpn2 & vpn_mask & !self.mask) == (query.vpn2 & vpn_mask & !self.mask)
    }
}
