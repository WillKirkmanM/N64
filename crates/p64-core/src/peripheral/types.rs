use super::PI_REGS_COUNT;
use crate::{builder::registers::Registers, N64};

const PAGE_SIZE_OFFSET: u32 = 2;
const PAGE_CYCLES: f64 = 5.0;
const BASE_CYCLES: f64 = 14.0;
const RCP_TO_CPU_CLOCK_RATIO: f64 = 1.5;
const DMA_BUFFER_BYTES: u32 = 128;
const DMA_PAGE_BYTES: u32 = 2048;
const DRAM_ALIGNMENT_BYTES: u32 = 8;
const FIFO_BUFFER_CYCLES: u64 = 28;
const PARTIAL_BYTE_CYCLES: u64 = 1;
const CPU_CLOCK_NUMERATOR: u64 = 3;
const CPU_CLOCK_DENOMINATOR: u64 = 2;

#[repr(u32)]
pub enum PiRegister {
    DramAddr = 0,
    CartAddr = 1,
    RdLen = 2,
    WrLen = 3,
    Status = 4,
    BsdDom1Lat = 5,
    BsdDom2Lat = 9,
    BsdDom2Rls = 12,
}

pub struct PiStatus;

impl PiStatus {
    pub const DMA_BUSY: u32 = 1 << 0;
    pub const IO_BUSY: u32 = 1 << 1;
    pub const INTERRUPT: u32 = 1 << 3;
    pub const RESET: u32 = 1 << 0;
    pub const CLR_INTR: u32 = 1 << 1;
}

pub struct Peripheral {
    pub regs: Registers,
}

impl Default for Peripheral {
    fn default() -> Self {
        Self {
            regs: Registers::new().with_size(PI_REGS_COUNT as usize).build(),
        }
    }
}

pub struct PiHandler {
    pub(super) read: fn(&mut N64, u32, u32, u32) -> u64,
    pub(super) write: fn(&mut N64, u32, u32, u32) -> u64,
}

#[derive(Copy, Clone)]
pub(super) enum PiDomain {
    Dom1 = 1,
    Dom2 = 2,
}

pub(super) struct DomainRegs {
    latency: u32,
    pulse_width: u32,
    release: u32,
    page_size: u32,
}

impl DomainRegs {
    pub(super) fn from_registers(regs: &Registers, domain: PiDomain) -> Self {
        let base: usize = match domain {
            PiDomain::Dom1 => PiRegister::BsdDom1Lat as usize,
            PiDomain::Dom2 => PiRegister::BsdDom2Lat as usize,
        };

        Self {
            latency: regs[base],
            pulse_width: regs[base + 1],
            release: regs[base + 3],
            page_size: regs[base + 2],
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u8)]
#[allow(dead_code)]
pub(super) enum JoyCommand {
    Status = 0x00,
    ControllerRead = 0x01,
    PakRead = 0x02,
    PakWrite = 0x03,
    Reset = 0xFF,
}

impl JoyCommand {
    pub(super) fn from_byte(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(Self::Status),
            0x01 => Some(Self::ControllerRead),
            0x02 => Some(Self::PakRead),
            0x03 => Some(Self::PakWrite),
            0xff => Some(Self::Reset),
            _ => None,
        }
    }
}

#[derive(Copy, Clone)]
pub struct PakHandler {
    pub read: fn(&mut N64, usize, u16, usize, usize),
    pub write: fn(&mut N64, usize, u16, usize, usize),
}

impl DomainRegs {
    pub(super) fn cycles(&self, length: u32) -> u64 {
        let page_size = 2.0_f64.powf((self.page_size + PAGE_SIZE_OFFSET) as f64);
        let total_pages = (length as f64 / page_size).ceil();

        let latency_cycles = (BASE_CYCLES + (self.latency + 1) as f64) * total_pages;
        let transfer_cycles =
            ((self.pulse_width + 1) as f64 + (self.release + 1) as f64) * (length as f64 / 2.0);
        let overhead_cycles = PAGE_CYCLES * total_pages;

        ((latency_cycles + transfer_cycles + overhead_cycles) * RCP_TO_CPU_CLOCK_RATIO) as u64
    }
}

#[derive(Default)]
struct PiTransferTiming {
    bytes: u64,
    pages: u64,
    buffers: u64,
    partial_bytes: u64,
}

impl PiTransferTiming {
    fn new(address: u32, bytes: u32, page_size: u32) -> Self {
        let bytes = u64::from(bytes).div_ceil(2) * 2;
        if bytes == 0 {
            return Self::default();
        }
        let page_bytes = 1u64 << (page_size + PAGE_SIZE_OFFSET);
        let offset = u64::from(crate::bits::align_down(address, 2)) % page_bytes;
        let pages = (offset + bytes - 2) / page_bytes + 1;
        let (buffers, partial_bytes) = if pages == 1 {
            if bytes == u64::from(DMA_BUFFER_BYTES) {
                (1, 0)
            } else {
                (0, bytes)
            }
        } else {
            let first_full = offset == 0;
            let end_offset = (offset + bytes) % page_bytes;
            let last_full = end_offset == 0;
            let buffers = u64::from(first_full)
                + u64::from(last_full)
                + (pages - 2) * page_bytes / u64::from(DMA_BUFFER_BYTES);
            let partial_bytes = if first_full { 0 } else { page_bytes - offset }
                + if last_full { 0 } else { end_offset };
            (buffers, partial_bytes)
        };
        Self {
            bytes,
            pages,
            buffers,
            partial_bytes,
        }
    }
}

impl DomainRegs {
    pub(super) fn transfer_cycles(&self, address: u32, bytes: u32) -> u64 {
        let transfer = PiTransferTiming::new(address, bytes, self.page_size);
        let latency = (BASE_CYCLES as u64 + u64::from(self.latency) + 1) * transfer.pages;
        let halfwords =
            (u64::from(self.pulse_width) + u64::from(self.release) + 2) * transfer.bytes / 2;
        let overhead =
            transfer.buffers * FIFO_BUFFER_CYCLES + transfer.partial_bytes * PARTIAL_BYTE_CYCLES;
        (latency + halfwords + overhead) * CPU_CLOCK_NUMERATOR / CPU_CLOCK_DENOMINATOR
    }
}

pub(crate) struct PiDmaLayout {
    pub source_length: u32,
    pub destination_length: u32,
    pub first_block: u32,
    pub misalignment: u32,
}

impl PiDmaLayout {
    pub(crate) fn new(dram_addr: u32, requested_length: u32) -> Self {
        let misalignment = crate::bits::alignment_offset(dram_addr, DRAM_ALIGNMENT_BYTES);
        let first_block = (DMA_BUFFER_BYTES - misalignment)
            .min(DMA_PAGE_BYTES - (crate::bits::alignment_offset(dram_addr, DMA_PAGE_BYTES)));
        // The first PI buffer is shortened by DRAM alignment and the 2 KiB page end.
        let rounding_threshold = if first_block < DRAM_ALIGNMENT_BYTES {
            first_block
        } else {
            first_block - 2
        };
        let source_length = if requested_length >= rounding_threshold {
            crate::bits::align_up(requested_length, 2)
        } else {
            requested_length
        };
        let mut destination_length = if source_length < first_block {
            source_length.saturating_sub(misalignment)
        } else {
            source_length
        };
        // A first block confined to the last DRAM doubleword carries its alignment
        // into the next buffer, delaying the tail by one misalignment-sized hole.
        if first_block < DRAM_ALIGNMENT_BYTES
            && destination_length > first_block + DMA_BUFFER_BYTES - misalignment
        {
            destination_length += misalignment;
        }

        Self {
            source_length,
            destination_length,
            first_block,
            misalignment,
        }
    }

    /// Skip the holes left by alignment at the first two PI buffer boundaries.
    pub(crate) fn source_offset(&self, destination: u32) -> Option<u32> {
        let first_hole = self.first_block.saturating_sub(self.misalignment);

        if (first_hole..self.first_block).contains(&destination) {
            return None;
        }

        let second_hole = self.first_block + DMA_BUFFER_BYTES - self.misalignment;

        if self.first_block < DRAM_ALIGNMENT_BYTES && destination >= second_hole {
            if destination < second_hole + self.misalignment {
                return None;
            }

            return Some(destination - self.misalignment);
        }

        Some(destination)
    }
}
