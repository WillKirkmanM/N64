use super::*;

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

#[derive(PartialEq)]
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

pub struct Memory {
    pub fast_read: [fn(&mut N64, u64, AccessSize) -> u32; 0x2000],
    pub memory_map_read: [fn(&mut N64, u64, AccessSize) -> u32; 0x2000],
    pub memory_map_write: [fn(&mut N64, u64, u32, u32); 0x2000],
    pub icache: [cache::ICache; 512],
    pub dcache: [cache::DCache; 512],
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            fast_read: [unmapped_read_mem; 0x2000],
            memory_map_read: [unmapped_read_mem; 0x2000],
            memory_map_write: [unmapped_write_mem; 0x2000],
            icache: [cache::ICache::default(); 512],
            dcache: [cache::DCache::default(); 512],
        }
    }
}
