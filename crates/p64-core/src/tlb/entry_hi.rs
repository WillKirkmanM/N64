pub(super) const VPN_SHIFT: u32 = 13;
pub(super) const REGION_SHIFT: u32 = 62;
pub(super) const EXTENDED_VPN_MASK: u64 = 0x7FF_FFFF;
pub(super) const COMPATIBLE_VPN_MASK: u64 = 0x7_FFFF;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct EntryHi {
    pub vpn2: u64,
    pub region: u8,
    pub asid: u8,
}

impl EntryHi {
    pub fn decode(raw: u64) -> Self {
        Self::for_address(raw, raw as u8)
    }

    pub(super) fn for_address(address: u64, asid: u8) -> Self {
        Self {
            vpn2: (address >> VPN_SHIFT) & EXTENDED_VPN_MASK,
            region: (address >> REGION_SHIFT) as u8,
            asid,
        }
    }

    pub(super) fn encode(self) -> u64 {
        (u64::from(self.region) << REGION_SHIFT) | (self.vpn2 << VPN_SHIFT) | u64::from(self.asid)
    }
}
