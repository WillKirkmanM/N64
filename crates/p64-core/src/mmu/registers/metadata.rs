use super::{MmuMask, MmuRegister};
use crate::tlb;

const RESET_CONFIG: u64 = 0x7006_e460;
const RESET_STATUS: u64 = 0x0040_0004;
const PROCESSOR_ID: u64 = 0xb22;
const RESET_CONTEXT: u64 = 0x007f_fff0;

impl MmuRegister {
    pub(in crate::mmu) fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Index),
            1 => Some(Self::Random),
            2 => Some(Self::EntryLo0),
            3 => Some(Self::EntryLo1),
            4 => Some(Self::Context),
            5 => Some(Self::PageMask),
            6 => Some(Self::Wired),
            8 => Some(Self::BadVAddr),
            9 => Some(Self::Count),
            10 => Some(Self::EntryHi),
            11 => Some(Self::Compare),
            12 => Some(Self::Status),
            13 => Some(Self::Cause),
            14 => Some(Self::EPC),
            15 => Some(Self::PrevID),
            16 => Some(Self::Config),
            17 => Some(Self::LLAddr),
            18 => Some(Self::WatchLo),
            19 => Some(Self::WatchHi),
            20 => Some(Self::XContext),
            26 => Some(Self::ParityErr),
            27 => Some(Self::CacheErr),
            28 => Some(Self::TagLo),
            29 => Some(Self::TagHi),
            30 => Some(Self::ErrorEPC),
            _ => None,
        }
    }

    pub(in crate::mmu) fn write_mask(self) -> u64 {
        match self {
            Self::Index => MmuMask::Index as u64,
            Self::EntryLo0 | Self::EntryLo1 => MmuMask::EntryLo as u64,
            Self::Context => MmuMask::Context as u64,
            Self::PageMask => MmuMask::PageMask as u64,
            Self::Wired => MmuMask::Wired as u64,
            Self::Count | Self::Compare | Self::LLAddr | Self::TagLo => u32::MAX as u64,
            Self::EntryHi => MmuMask::EntryHi as u64,
            Self::Status => MmuMask::Status as u64,
            Self::Cause => MmuMask::Cause as u64,
            Self::EPC | Self::ErrorEPC => u64::MAX,
            Self::Config => MmuMask::Config as u64,
            Self::WatchLo => MmuMask::WatchLo as u64,
            Self::WatchHi => MmuMask::WatchHi as u64,
            Self::XContext => MmuMask::XContext as u64,
            Self::ParityErr => MmuMask::ParityErr as u64,
            Self::Random
            | Self::BadVAddr
            | Self::PrevID
            | Self::CacheErr
            | Self::TagHi
            | Self::RegsCount => 0,
        }
    }

    /// Only these registers are initialised; existing values in other slots survive.
    pub(in crate::mmu) fn reset_value(self) -> Option<u64> {
        match self {
            Self::Random => Some((tlb::ENTRY_COUNT - 1) as u64),
            Self::Config => Some(RESET_CONFIG),
            Self::Status => Some(RESET_STATUS),
            Self::PrevID => Some(PROCESSOR_ID),
            Self::EPC | Self::ErrorEPC => Some(u64::MAX),
            Self::BadVAddr => Some(u32::MAX as u64),
            Self::Context => Some(RESET_CONTEXT),
            _ => None,
        }
    }
}
