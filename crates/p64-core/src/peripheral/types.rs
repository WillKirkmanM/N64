use super::*;

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

pub(super) const RCP_TO_CPU_CLOCK_RATIO: f64 = 1.5;

pub(super) const BASE_CYCLES: f64 = 14.0;

pub(super) const PAGE_CYCLES: f64 = 5.0;

pub(super) const PAGE_SIZE_OFFSET: u32 = 2;

pub struct Peripheral {
    pub regs: Registers,
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
    pub(super) latency: u32,
    pub(super) pulse_width: u32,
    pub(super) release: u32,
    pub(super) page_size: u32,
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

#[derive(Copy, Clone)]
pub struct PakHandler {
    pub read: fn(&mut N64, usize, u16, usize, usize),
    pub write: fn(&mut N64, usize, u16, usize, usize),
}

impl PiStatus {
    pub const DMA_BUSY: u32 = 1 << 0;
    pub const IO_BUSY: u32 = 1 << 1;
    pub const INTERRUPT: u32 = 1 << 3;
    pub const RESET: u32 = 1 << 0;
    pub const CLR_INTR: u32 = 1 << 1;
}

impl Default for Peripheral {
    fn default() -> Self {
        Self {
            regs: Registers::new().with_size(PI_REGS_COUNT as usize).build(),
        }
    }
}

impl DomainRegs {
    pub(super) fn from_device(n64: &N64, domain: PiDomain) -> Self {
        let base: usize = match domain {
            PiDomain::Dom1 => PiRegister::BsdDom1Lat as usize,
            PiDomain::Dom2 => PiRegister::BsdDom2Lat as usize,
        };

        Self {
            latency: n64.peripheral.regs[base],
            pulse_width: n64.peripheral.regs[base + 1],
            release: n64.peripheral.regs[base + 2],
            page_size: n64.peripheral.regs[base + 3],
        }
    }
}
