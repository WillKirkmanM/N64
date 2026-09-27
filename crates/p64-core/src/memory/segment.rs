const KSEG_MASK: u64 = 0xc0000000;
const KSEG0_BASE: u64 = 0x80000000;
const CACHE_MASK: u64 = 0x20000000;
const PHYSICAL_MASK: u64 = 0x1FFFFFFF;
const REGION_SHIFT: u32 = 62;
const CACHE_ATTRIBUTE_SHIFT: u32 = 59;
const CACHE_ATTRIBUTE_MASK: u64 = 7;
const UNCACHED_ATTRIBUTE: u64 = 2;
const XKPHYS_RESERVED_MASK: u64 = 0x07FF_FFFF_0000_0000;
const XKPHYS_ADDRESS_MASK: u64 = u32::MAX as u64;
const EXTENDED_MAPPED_RESERVED_MASK: u64 = 0x3FFF_FF00_0000_0000;
const XKSEG_END: u64 = 0xC000_00FF_7FFF_FFFF;

pub(super) enum AddressSegment {
    Direct { physical: u64, cached: bool },
    Mapped,
    Forbidden,
}

impl AddressSegment {
    pub(super) fn classify(status: crate::mmu::Status, address: u64) -> Self {
        let mode = status.mode();
        let extended = status.extended_addressing();

        if address == address as i32 as i64 as u64 {
            let low = address as u32;
            let permitted = match low {
                0..=0x7FFF_FFFF => true,
                0xC000_0000..=0xDFFF_FFFF => mode <= 1,
                _ => mode == 0,
            };

            if !permitted {
                return Self::Forbidden;
            }

            if low < 0x8000_0000 && status.error_level() {
                return Self::Direct {
                    physical: low as u64,
                    cached: false,
                };
            }

            if address & KSEG_MASK == KSEG0_BASE {
                return Self::Direct {
                    physical: address & PHYSICAL_MASK,
                    cached: address & CACHE_MASK == 0,
                };
            }

            return Self::Mapped;
        }

        if !extended {
            return Self::Forbidden;
        }

        let region = address >> REGION_SHIFT;

        if region == 2 {
            return if mode == 0 && address & XKPHYS_RESERVED_MASK == 0 {
                Self::Direct {
                    physical: address & XKPHYS_ADDRESS_MASK,
                    cached: (address >> CACHE_ATTRIBUTE_SHIFT) & CACHE_ATTRIBUTE_MASK
                        != UNCACHED_ATTRIBUTE,
                }
            } else {
                Self::Forbidden
            };
        }

        let permitted = address & EXTENDED_MAPPED_RESERVED_MASK == 0
            && (region == 0
                || (region == 1 && mode <= 1)
                || (region == 3 && mode == 0 && address <= XKSEG_END));

        if permitted {
            Self::Mapped
        } else {
            Self::Forbidden
        }
    }
}
