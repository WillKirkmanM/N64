mod bus;
mod caches;
mod prepared_code;

pub use bus::MemoryBus;
pub use caches::MemoryCaches;
pub use prepared_code::PreparedCode;

use crate::cache;
use serde::{Deserialize, Serialize};

#[repr(usize)]
pub enum MemoryMap {
    RdramDram = 0x00000000,
    RspMem = 0x04000000,
    RspRegs = 0x04040000,
    RspRegsPc = 0x04080000,
    DpcRegs = 0x04100000,
    DpsRegs = 0x04200000,
    MiRegs = 0x04300000,
    ViRegs = 0x04400000,
    AiRegs = 0x04500000,
    PiRegs = 0x04600000,
    RiRegs = 0x04700000,
    SerialRegisters = 0x04800000,
    Dom2Addr2 = 0x08000000,
    CartRom = 0x10000000,
    PifMem = 0x1fc00000,
}

#[derive(Copy, Clone, PartialEq)]
pub enum AccessType {
    Write,
    Read,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum AccessSize {
    Word = 4,
    Dword = 8,
    Dcache = 16,
    Icache = 32,
}

#[derive(Default)]
pub struct Memory {
    pub bus: MemoryBus,
    pub caches: MemoryCaches,
    pub prepared_code: PreparedCode,
    pub fetch_pipeline: cache::FetchPipeline,
}

impl Memory {
    /// Discard prepared entry handles when guest code or its engine changes.
    /// Keep host code pointers out of the guest instruction cache itself.
    pub(crate) fn clear_native_line(&mut self, line_index: usize) {
        self.prepared_code.clear_line(line_index);
    }
}
