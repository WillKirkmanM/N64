use crate::cache::{DCache, ICache};

const CACHE_LINE_COUNT: usize = 512;

pub struct MemoryCaches {
    pub icache: [ICache; CACHE_LINE_COUNT],
    pub dcache: [DCache; CACHE_LINE_COUNT],
}

impl Default for MemoryCaches {
    fn default() -> Self {
        Self {
            icache: [ICache::default(); CACHE_LINE_COUNT],
            dcache: [DCache::default(); CACHE_LINE_COUNT],
        }
    }
}
