use crate::{
    audio::{handle_ai_register_read, handle_ai_register_write},
    cache, cartridge, mips, peripheral, pif, rdp, rdram, rsp, serial, sram, video,
};

use super::{
    cache::{dcache_read, dcache_write},
    exceptions, tlb, N64,
};

const MASK_64K: usize = 0xFFFF;
const UNMAPPED_BUS_VALUE: u32 = 3;
const RDRAM_BUS_END: usize = 0x03EF_FFFF;
const RSP_MEMORY_WINDOW_MASK: usize = 0x3_FFFF;
const RDRAM_REGISTERS_START: usize = 0x03F0_0000;
const RDRAM_REGISTERS_END: usize = 0x03FF_FFFF;
const CARTRIDGE_BUS_END: usize = 0x1FBF_FFFF;
const IS_VIEWER_START: usize = 0x13FF_0000;
const IS_VIEWER_END: usize = 0x13FF_FFFF;

mod segment;
mod types;
use segment::AddressSegment;
pub use types::{AccessSize, AccessType, Memory, MemoryBus, MemoryCaches, MemoryMap, PreparedCode};

pub fn unmapped_read_mem(_device: &mut N64, _address: u64, _access_size: AccessSize) -> u32 {
    UNMAPPED_BUS_VALUE
}

pub fn unmapped_write_mem(_device: &mut N64, _address: u64, _value: u32, _mask: u32) {}

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

pub fn masked_write_32(dst: &mut u32, value: u32, mask: u32) {
    *dst = crate::bits::replace_bits(*dst, value, mask)
}

pub fn masked_write_64(dst: &mut u64, value: u64, mask: u64) {
    *dst = crate::bits::replace_bits(*dst, value, mask)
}

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

pub fn translate_address(
    n64: &mut N64,
    address: u64,
    access_type: AccessType,
) -> (u64, bool, bool) {
    n64.cpu.execution.data_address = address;

    match AddressSegment::classify(n64.cp0().status(), address) {
        AddressSegment::Direct { physical, cached } => (physical, cached, false),
        AddressSegment::Mapped => tlb::get_physical_address(n64, address, access_type),
        AddressSegment::Forbidden => {
            exceptions::address_error_exception(n64, address, access_type);

            (0, false, true)
        }
    }
}

/// Nonfaulting instruction translation never changes CP0 or the fetch address.
pub fn peek_translation(n64: &N64, address: u64) -> Option<(u64, bool)> {
    if !crate::bits::is_aligned(address, AccessSize::Word as u64) {
        return None;
    }

    match AddressSegment::classify(n64.cp0().status(), address) {
        AddressSegment::Direct { physical, cached } => Some((physical, cached)),
        AddressSegment::Forbidden => None,
        AddressSegment::Mapped => {
            let (entry, odd) = tlb::lookup_entry(n64, address)?;
            let page = entry.page(odd);

            page.valid
                .then(|| (page.translate(address, entry.page_size()), page.cacheable()))
        }
    }
}

#[inline]
fn get_handler_index(phys_address: u64) -> usize {
    MemoryBus::region_index(phys_address)
}

pub fn read_data(n64: &mut N64, addr: u64, size: AccessSize, cached: bool) -> u32 {
    match cached {
        true => dcache_read(n64, addr),
        false => {
            let read = n64
                .memory
                .bus
                .memory_map_read
                .get(get_handler_index(addr))
                .copied()
                .unwrap_or(unmapped_read_mem);

            read(n64, addr, size)
        }
    }
}

/// Read both big-endian words through the guest bus, preserving access timing.
pub fn read_doubleword(n64: &mut N64, address: u64, cached: bool) -> u64 {
    let high = read_data(n64, address, AccessSize::Dword, cached) as u64;
    let low = read_data(n64, address + 4, AccessSize::Dword, cached) as u64;

    (high << 32) | low
}

pub fn write_data(n64: &mut N64, addr: u64, value: u32, mask: u32, cached: bool) {
    match cached {
        true => dcache_write(n64, addr, value, mask),
        false => {
            // Uncached stores enter the external write buffer. An instruction
            // refill already issued by IC/RF is serviced before this store is
            // visible to that refill. DC cache writebacks retain higher priority.
            if addr < rdram::RDRAM_SIZE as u64 {
                cache::service_pending_fetch(n64);
            }

            let write = n64
                .memory
                .bus
                .memory_map_write
                .get(get_handler_index(addr))
                .copied()
                .unwrap_or(unmapped_write_mem);

            write(n64, addr, value, mask)
        }
    }
}

pub fn initialise_memory_map(n64: &mut N64) {
    n64.memory.bus.map_range(
        MemoryMap::RdramDram as usize,
        RDRAM_BUS_END,
        rdram::read_mem,
        rdram::write_mem,
        Some(rdram::read_mem_fast),
    );
    n64.memory.bus.map_range(
        MemoryMap::RspMem as usize,
        MemoryMap::RspMem as usize + RSP_MEMORY_WINDOW_MASK,
        rsp::read_mem,
        rsp::write_mem,
        Some(rsp::read_mem_fast),
    );
    n64.memory.bus.map_range(
        RDRAM_REGISTERS_START,
        RDRAM_REGISTERS_END,
        rdram::read_rdram_regs,
        rdram::write_rdram_regs,
        Some(rdram::read_rdram_regs),
    );
    n64.memory.bus.map_range(
        MemoryMap::CartRom as usize,
        CARTRIDGE_BUS_END,
        cartridge::read_mem,
        cartridge::write_mem,
        Some(cartridge::read_mem_fast),
    );
    n64.memory.bus.map_range(
        MemoryMap::Dom2Addr2 as usize,
        MemoryMap::CartRom as usize - 1,
        sram::read_mem,
        sram::write_mem,
        Some(sram::read_mem),
    );
    n64.memory.bus.map_range(
        IS_VIEWER_START,
        IS_VIEWER_END,
        crate::is_viewer::read_mem,
        crate::is_viewer::write_mem,
        Some(crate::is_viewer::read_mem),
    );

    n64.memory.bus.map_range(
        MemoryMap::RspRegs as usize,
        MemoryMap::RspRegs as usize + MASK_64K,
        rsp::read_regs,
        rsp::write_regs,
        None,
    );
    n64.memory.bus.map_range(
        MemoryMap::RspRegsPc as usize,
        MemoryMap::RspRegsPc as usize + MASK_64K,
        rsp::read_regs2,
        rsp::write_regs2,
        None,
    );
    n64.memory.bus.map_range(
        MemoryMap::DpcRegs as usize,
        MemoryMap::DpcRegs as usize + MASK_64K,
        rdp::read_regs_dpc,
        rdp::write_regs_dpc,
        None,
    );
    n64.memory.bus.map_range(
        MemoryMap::DpsRegs as usize,
        MemoryMap::DpsRegs as usize + MASK_64K,
        rdp::read_regs_dps,
        rdp::write_regs_dps,
        None,
    );
    n64.memory.bus.map_range(
        MemoryMap::MiRegs as usize,
        MemoryMap::MiRegs as usize + MASK_64K,
        mips::read_regs,
        mips::write_regs,
        None,
    );
    n64.memory.bus.map_range(
        MemoryMap::ViRegs as usize,
        MemoryMap::ViRegs as usize + MASK_64K,
        video::read_regs,
        video::write_regs,
        None,
    );
    n64.memory.bus.map_range(
        MemoryMap::AiRegs as usize,
        MemoryMap::AiRegs as usize + MASK_64K,
        handle_ai_register_read,
        handle_ai_register_write,
        None,
    );
    n64.memory.bus.map_range(
        MemoryMap::PiRegs as usize,
        MemoryMap::PiRegs as usize + MASK_64K,
        peripheral::read_regs,
        peripheral::write_regs,
        None,
    );
    n64.memory.bus.map_range(
        MemoryMap::RiRegs as usize,
        MemoryMap::RiRegs as usize + MASK_64K,
        rdram::read_regs,
        rdram::write_regs,
        None,
    );
    n64.memory.bus.map_range(
        MemoryMap::SerialRegisters as usize,
        MemoryMap::SerialRegisters as usize + MASK_64K,
        serial::read_regs,
        serial::write_regs,
        None,
    );
    n64.memory.bus.map_range(
        MemoryMap::PifMem as usize,
        MemoryMap::PifMem as usize + MASK_64K,
        pif::read_mem,
        pif::write_mem,
        None,
    );

    cache::init(n64)
}
