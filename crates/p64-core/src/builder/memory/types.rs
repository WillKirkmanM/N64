use super::{MemoryAccess, MemoryData, MemoryLayout, MemoryTiming};

pub type WriteCallback = fn(address: u64, value: u32) -> bool;

#[derive(Default, Clone)]
pub struct Memory {
    pub data: MemoryData,
    pub layout: MemoryLayout,
    pub access: MemoryAccess,
    pub timing: MemoryTiming,
}
