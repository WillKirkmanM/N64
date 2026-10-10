const CACHE_INDEX_MASK: u64 = 0x1ff;
const OPERATION_SHIFT: u32 = 2;

mod selector;
pub(super) use selector::{CacheBank, CacheSelector};

#[derive(Clone, Copy)]
pub(super) enum CacheOperation {
    IndexInvalidate,
    IndexLoadTag,
    IndexStoreTag,
    CreateDirtyExclusive,
    HitInvalidate,
    FillOrWritebackInvalidate,
    HitWriteback,
}

impl CacheOperation {
    fn decode(selector: usize) -> Option<Self> {
        match selector >> OPERATION_SHIFT {
            0 => Some(Self::IndexInvalidate),
            1 => Some(Self::IndexLoadTag),
            2 => Some(Self::IndexStoreTag),
            3 if selector == 0x0d => Some(Self::CreateDirtyExclusive),
            4 => Some(Self::HitInvalidate),
            5 => Some(Self::FillOrWritebackInvalidate),
            6 => Some(Self::HitWriteback),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct CacheAddress {
    pub physical: u64,
    pub index: usize,
}

impl CacheAddress {
    pub fn new(physical: u64, virtual_address: u64, line_shift: u64) -> Self {
        Self {
            physical,
            index: ((virtual_address >> line_shift) & CACHE_INDEX_MASK) as usize,
        }
    }

    pub fn page_tag(self) -> u32 {
        crate::bits::align_down(self.physical, crate::cache::TAG_PAGE_BYTES) as u32
    }
}
