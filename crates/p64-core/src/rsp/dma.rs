use super::{RspDma, ADDR_MASK};

pub(super) const IMEM_SELECT: u32 = 0x1000;
pub(super) const DMA_LENGTH_READBACK: u32 = 0xff8;
pub(super) const DMA_START_CYCLES: u64 = 9;
const DMA_ALIGNMENT_MASK: u32 = 7;
const DMA_COUNT_SHIFT: u32 = 12;
const DMA_COUNT_MASK: u32 = 0xff;
const DMA_SKIP_SHIFT: u32 = 20;
const DMA_MEMORY_ADDRESS_MASK: u32 = ADDR_MASK & !DMA_ALIGNMENT_MASK;
const DMA_DRAM_ADDRESS_MASK: u32 = 0xff_fff8;

pub(super) struct DmaLayout {
    pub(super) length: u32,
    pub(super) count: u32,
    pub(super) skip: u32,
    pub(super) memory_address: u32,
    pub(super) dram_address: u32,
    pub(super) bank: u32,
}

impl DmaLayout {
    pub(super) fn decode(command: &RspDma) -> Self {
        Self {
            length: ((command.length & ADDR_MASK) | DMA_ALIGNMENT_MASK) + 1,
            count: ((command.length >> DMA_COUNT_SHIFT) & DMA_COUNT_MASK) + 1,
            skip: (command.length >> DMA_SKIP_SHIFT) & DMA_MEMORY_ADDRESS_MASK,
            memory_address: command.memaddr & DMA_MEMORY_ADDRESS_MASK,
            dram_address: command.dramaddr & DMA_DRAM_ADDRESS_MASK,
            bank: command.memaddr & IMEM_SELECT,
        }
    }
}
