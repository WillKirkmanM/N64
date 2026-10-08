use serde::{Deserialize, Serialize};

use crate::{audio::{handle_ai_register_read, handle_ai_register_write}, cache, cartridge, mips, peripheral, pif, rdp, rdram, rsp, serial, video};

use super::{cache::{dcache_read, dcache_write}, tlb, N64};

const KSEG_MASK: u64 = 0xc0000000;
const KSEG0_BASE: u64 = 0x80000000;
const CACHE_MASK: u64 = 0x20000000;
const PHYSICAL_MASK: u64 = 0x1FFFFFFF;
const MASK_64K: usize = 0xFFFF;

pub fn unmapped_read_mem(_device: &mut N64, _address: u64, _access_size: AccessSize) -> u32 {
    3
}
pub fn unmapped_write_mem(_device: &mut N64, _address: u64, _value: u32, _mask: u32) {}

pub fn masked_write_32(dst: &mut u32, value: u32, mask: u32) {
    *dst = (*dst & !mask) | (value & mask)
}
pub fn masked_write_64(dst: &mut u64, value: u64, mask: u64) {
    *dst = (*dst & !mask) | (value & mask)
}

pub fn translate_address(
    n64: &mut N64,
    address: u64,
    access_type: AccessType,
) -> (u64, bool, bool) {
    match address & KSEG_MASK {
        addr if addr != KSEG0_BASE => tlb::get_physical_address(n64, address, access_type),
        _ => (address & PHYSICAL_MASK, (address & CACHE_MASK) == 0, false),
    }
}

#[inline]
fn get_handler_index(phys_address: u64) -> usize {
    (phys_address >> 16) as usize
}

pub fn read_data(n64: &mut N64, addr: u64, size: AccessSize, cached: bool) -> u32 {
    match cached {
        true => dcache_read(n64, addr),
        false => n64.memory.bus.memory_map_read[get_handler_index(addr)](n64, addr, size),
    }
}

pub fn write_data(n64: &mut N64, addr: u64, value: u32, mask: u32, cached: bool) {
    match cached {
        true => dcache_write(n64, addr, value, mask),
        false => n64.memory.bus.memory_map_write[get_handler_index(addr)](n64, addr, value, mask),
    }
}

macro_rules! map_memory_range {
    ($n64: expr, $start:expr, $end:expr, $read:path, $write:path $(, $fast_read:path)?) => {
        for i in ($start as usize >> 16)..=($end as usize >> 16) {
            $n64.memory.bus.memory_map_read[i] = $read;
            $n64.memory.bus.memory_map_write[i] = $write;
            $($n64.memory.bus.fast_read[i] = $fast_read)?
        }
    };
}

pub fn initialise_memory_map(n64: &mut N64) {
    map_memory_range!(
        n64,
        MemoryMap::RdramDram,
        MemoryMap::RdramDram as usize + 0x03EFFFFF,
        rdram::read_mem_fast,
        rdram::write_mem,
        rdram::read_mem
    );
    map_memory_range!(
        n64,
        MemoryMap::RspMem,
        MemoryMap::RspMem as usize + 0x3FFFF,
        rsp::read_mem,
        rsp::write_mem,
        rsp::read_mem_fast
    );
    map_memory_range!(
        n64,
        MemoryMap::CartRom,
        MemoryMap::CartRom as usize + n64.cartridge.rom.len() - 1,
        cartridge::read_mem,
        cartridge::write_mem,
        cartridge::read_mem_fast
    );

    map_memory_range!(
        n64,
        MemoryMap::RspRegs,
        MemoryMap::RspRegs as usize + MASK_64K,
        rsp::read_regs,
        rsp::write_regs
    );
    map_memory_range!(
        n64,
        MemoryMap::RspRegsPc,
        MemoryMap::RspRegsPc as usize + MASK_64K,
        rsp::read_regs2,
        rsp::write_regs2
    );
    map_memory_range!(
        n64,
        MemoryMap::DpcRegs,
        MemoryMap::DpcRegs as usize + MASK_64K,
        rdp::read_regs_dpc,
        rdp::write_regs_dpc
    );
    map_memory_range!(
        n64,
        MemoryMap::DpsRegs,
        MemoryMap::DpsRegs as usize + MASK_64K,
        rdp::read_regs_dps,
        rdp::write_regs_dps
    );
    map_memory_range!(
        n64,
        MemoryMap::MiRegs,
        MemoryMap::MiRegs as usize + MASK_64K,
        mips::read_regs,
        mips::write_regs
    );
    map_memory_range!(
        n64,
        MemoryMap::ViRegs,
        MemoryMap::ViRegs as usize + MASK_64K,
        video::read_regs,
        video::write_regs
    );
    map_memory_range!(
        n64,
        MemoryMap::AiRegs,
        MemoryMap::AiRegs as usize + MASK_64K,
        handle_ai_register_read,
        handle_ai_register_write
    );
    map_memory_range!(
        n64,
        MemoryMap::PiRegs,
        MemoryMap::PiRegs as usize + MASK_64K,
        peripheral::read_regs,
        peripheral::write_regs
    );
    map_memory_range!(
        n64,
        MemoryMap::RiRegs,
        MemoryMap::RiRegs as usize + MASK_64K,
        rdram::read_regs,
        rdram::write_regs
    );
    map_memory_range!(
        n64,
        MemoryMap::SerialRegisters,
        MemoryMap::SerialRegisters as usize + MASK_64K,
        serial::read_regs,
        serial::write_regs
    );
    map_memory_range!(
        n64,
        MemoryMap::PifMem,
        MemoryMap::PifMem as usize + MASK_64K,
        pif::read_mem,
        pif::write_mem
    );

    cache::init(n64)
}

mod types;
pub use types::{MemoryMap, AccessType, AccessSize, Memory};

mod segment;
use segment::AddressSegment;

pub use types::{MemoryBus, MemoryCaches, PreparedCode};

pub fn reverse_endian(n64: &N64) -> bool {
    let status = n64.cp0().status();

    status.mode() == 2 && status.reverse_endian()
}

pub fn reverse_endian_address(n64: &N64, address: u64, bytes: u64) -> u64 {
    if reverse_endian(n64) {
        address ^ (8 - bytes)
    } else {
        address
    }
}

/// Read both big-endian words through the guest bus, preserving access timing.
pub fn read_doubleword(n64: &mut N64, address: u64, cached: bool) -> u64 {
    let high = read_data(n64, address, AccessSize::Dword, cached) as u64;
    let low = read_data(n64, address + 4, AccessSize::Dword, cached) as u64;

    (high << 32) | low
}

pub fn peek_translation(n64: &N64, address: u64) -> Option<(u64, bool)> {
    if !crate::bits::is_aligned(address, AccessSize::Word as u64) {
        return None;
    }
    match segment::AddressSegment::classify(n64.cp0().status(), address) {
        segment::AddressSegment::Direct { physical, cached } => Some((physical, cached)),
        segment::AddressSegment::Forbidden => None,
        segment::AddressSegment::Mapped => {
            let address = address & 0xffff_ffff;
            let entry = n64.cpu.mmu.tlb_lut_r.get((address >> 12) as usize)?;
            (entry.address != 0).then_some((
                (entry.address & 0x1ffff000) | (address & 0xfff),
                entry.cached,
            ))
        }
    }
}

pub(crate) fn read_rdram_byte(n64: &N64, address: u32) -> u8 {
    let offset = (address as usize & rdram::RDRAM_MASK) ^ n64.byte_swap;
    n64.rdram.mem.data.get(offset).copied().unwrap_or(0)
}

pub(crate) fn write_rdram_byte(n64: &mut N64, address: u32, value: u8) {
    #[cfg(feature = "accuracy-diagnostics")]
    rdram::trace_write(n64, "bus-byte", u64::from(address), u32::from(value), 0xff);
    let offset = (address as usize & rdram::RDRAM_MASK) ^ n64.byte_swap;
    if let Some(destination) = n64.rdram.mem.data.get_mut(offset) {
        *destination = value;
        rdram::record_write(n64, u64::from(address) & rdram::RDRAM_MASK as u64);
    }
}
