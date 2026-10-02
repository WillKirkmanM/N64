mod native_preparation;

pub use native_preparation::NativePreparation;

use crate::N64;

const CP0_TAG_ADDRESS_MASK: u64 = 0x0FFF_FF00;
const CP0_TAG_VALID: u64 = 1 << 7;
const CP0_TAG_DIRTY: u64 = 1 << 6;

#[derive(Clone, Copy)]
pub(super) struct FetchLatch {
    pub(super) virtual_address: u64,
    pub(super) physical_address: u64,
    pub(super) word: u32,
    pub(super) instruction: fn(&mut N64, u32),
}

/// IC/RF lookahead is independent of cache validity once an instruction has
/// entered the pipeline. Pending RF misses are arbitrated after DC operations.
/// See NEC VR4300 User's Manual sections 4.6.3, 4.6.8, 4.7.4 and 4.9:
/// https://hack64.net/docs/VR43XX.pdf
pub struct FetchPipeline {
    pub enabled: bool,
    pub native_min_block_length: usize,
    pub(super) active: bool,
    pub(super) next: Option<FetchLatch>,
    pub(super) pending_refill: Option<(u64, u64)>,
}

impl Default for FetchPipeline {
    fn default() -> Self {
        Self {
            enabled: !std::env::var("N64_PIPELINE_CACHE").is_ok_and(|value| value == "0"),
            native_min_block_length: std::env::var("N64_NATIVE_MIN_BLOCK_LENGTH")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(2)
                .clamp(2, 8),
            active: false,
            next: None,
            pending_refill: None,
        }
    }
}

#[derive(Copy, Clone)]
pub struct ICache {
    pub valid: bool,
    pub tag: u32,
    pub index: u16,
    pub words: [u32; 8],
    pub instruction: [fn(&mut N64, u32); 8],
    pub native: NativePreparation,
}

impl Default for ICache {
    fn default() -> Self {
        fn default_instruction(_device: &mut N64, _value: u32) {}

        ICache {
            valid: false,
            tag: 0,
            index: 0,
            words: [0; 8],
            instruction: [default_instruction; 8],
            native: NativePreparation::default(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct DCache {
    pub valid: bool,
    pub dirty: bool,
    pub store_dirty: bool,
    pub tag: u32,
    pub index: u16,
    pub words: [u32; 4],
}

impl ICache {
    #[inline]
    pub(crate) fn line_index(address: u64) -> usize {
        ((address >> super::LINE_INDEX_SHIFT_ICACHE) & super::LINE_INDEX_MASK_ICACHE) as usize
    }

    #[inline]
    pub(crate) fn word_index(address: u64) -> usize {
        ((address >> super::WORD_INDEX_SHIFT_ICACHE) & super::WORD_INDEX_MASK_ICACHE) as usize
    }

    pub(crate) fn cp0_tag(&self) -> u64 {
        ((self.tag >> 4) as u64 & CP0_TAG_ADDRESS_MASK) | if self.valid { CP0_TAG_VALID } else { 0 }
    }

    pub(crate) fn set_cp0_tag(&mut self, tag: u64) {
        self.valid = tag & CP0_TAG_VALID != 0;
        self.tag = ((tag & CP0_TAG_ADDRESS_MASK) << 4) as u32;
    }
}

impl DCache {
    #[inline]
    pub(crate) fn line_index(address: u64) -> usize {
        ((address >> super::LINE_INDEX_SHIFT_DCACHE) & super::LINE_INDEX_MASK_DCACHE) as usize
    }

    #[inline]
    pub(crate) fn word_index(address: u64) -> usize {
        ((address >> super::WORD_INDEX_SHIFT_DCACHE) & super::WORD_INDEX_MASK_DCACHE) as usize
    }

    pub(crate) fn cp0_tag(&self) -> u64 {
        ((self.tag >> 4) as u64 & CP0_TAG_ADDRESS_MASK)
            | if self.valid { CP0_TAG_VALID } else { 0 }
            | if self.valid && self.dirty {
                CP0_TAG_DIRTY
            } else {
                0
            }
    }

    pub(crate) fn set_cp0_tag(&mut self, tag: u64) {
        self.valid = tag & CP0_TAG_VALID != 0;
        self.dirty = tag & CP0_TAG_DIRTY != 0;
        self.tag = ((tag & CP0_TAG_ADDRESS_MASK) << 4) as u32;
    }
}

#[derive(Default)]
pub(super) struct NativeEntryPlan {
    pub(super) integer_length: usize,
    pub(super) memory_length: usize,
    pub(super) branch_length: Option<usize>,
}

impl NativeEntryPlan {
    pub(super) fn is_candidate(&self) -> bool {
        self.integer_length != 0 || self.memory_length != 0 || self.branch_length.is_some()
    }
}
