use crate::builder::registers::register_index;
mod types;
pub use types::*;
mod joybus;
pub use joybus::process;
use types::{DomainRegs, JoyCommand, PiDomain};

use super::{
    events::{create_event, EventType},
    memory::{self, MemoryMap},
    mmu::MmuRegister,
    N64,
};
use crate::{cartridge, mips, sram};

pub const PI_REGS_COUNT: u32 = 13;

const CONTROLLER_FLAGS: u16 = 0x0005;

const PAK_CHUNK_SIZE: usize = 0x20;
const DMA_LENGTH_MASK: u32 = 0x00FF_FFFF;
const CART_ADDRESS_MASK: u32 = 0xFFFF_FFFE;
const DRAM_ADDRESS_MASK: u32 = 0x00FF_FFFE;
const DMA_LENGTH_READBACK: u32 = 0x7F;
const DOMAIN_REGISTER_COUNT: u32 = 4;
const DOMAIN_PAGE_SIZE_REGISTER: u32 = 2;
const DOMAIN_RELEASE_REGISTER: u32 = 3;
const DOMAIN_PAGE_SIZE_MASK: u32 = 0xF;
const DOMAIN_RELEASE_MASK: u32 = 0x3;
const PI_STATUS_ERROR: u32 = 1 << 2;

pub fn read_regs(n64: &mut N64, address: u64, _access_size: memory::AccessSize) -> u32 {
    let reg = register_index(address);
    if reg >= PI_REGS_COUNT as usize {
        return 0;
    }

    match reg as u32 {
        x if x == PiRegister::WrLen as u32 || x == PiRegister::RdLen as u32 => DMA_LENGTH_READBACK,
        x if x == PiRegister::CartAddr as u32 => n64.peripheral.regs[reg] & CART_ADDRESS_MASK,
        x if x == PiRegister::DramAddr as u32 => n64.peripheral.regs[reg] & DRAM_ADDRESS_MASK,
        _ => n64.peripheral.regs[reg],
    }
}

pub(crate) fn cart_dma_layout(dram_addr: u32, requested_length: u32) -> PiDmaLayout {
    PiDmaLayout::new(dram_addr, requested_length)
}

fn dma_transfer(n64: &mut N64, is_read: bool) {
    let handler = get_handler(n64.peripheral.regs[PiRegister::CartAddr as usize]);

    let cart_addr = crate::bits::align_down(n64.peripheral.regs[PiRegister::CartAddr as usize], 2);
    let dram_addr = n64.peripheral.regs[PiRegister::DramAddr as usize] & DRAM_ADDRESS_MASK;
    let mut length = (n64.peripheral.regs[if is_read {
        PiRegister::RdLen
    } else {
        PiRegister::WrLen
    } as usize]
        & DMA_LENGTH_MASK)
        + 1;

    let destination_length;

    if !is_read && cart_addr >= MemoryMap::CartRom as u32 {
        let layout = cart_dma_layout(dram_addr, length);
        length = layout.source_length;
        destination_length = layout.destination_length;
    } else {
        if length >= DMA_LENGTH_READBACK && (length & 1) != 0 {
            length += 1;
        }
        destination_length = length;
    }

    let cycles = if is_read {
        (handler.read)(n64, cart_addr, dram_addr, length)
    } else {
        (handler.write)(n64, cart_addr, dram_addr, length)
    };

    create_event(
        n64,
        EventType::PI,
        n64.cp0_register(MmuRegister::Count) + cycles,
        dma_event,
    );

    n64.peripheral.regs[PiRegister::DramAddr as usize] =
        crate::bits::align_up(dram_addr + destination_length, 8);
    n64.peripheral.regs[PiRegister::CartAddr as usize] =
        crate::bits::align_up_wrapping(cart_addr.wrapping_add(length), 2);

    n64.peripheral.regs[PiRegister::Status as usize] |= PiStatus::DMA_BUSY;
}

pub fn dma_read(n64: &mut N64) {
    dma_transfer(n64, true);
}

pub fn dma_write(n64: &mut N64) {
    dma_transfer(n64, false);
}

pub fn get_handler(address: u32) -> PiHandler {
    let mut handler = PiHandler {
        read: cartridge::dma_read,
        write: cartridge::dma_write,
    };

    match address {
        addr if addr >= MemoryMap::CartRom as u32 => {
            handler.read = cartridge::dma_read;
            handler.write = cartridge::dma_write;
        }
        addr if addr >= MemoryMap::Dom2Addr2 as u32 => {
            handler.read = sram::dma_read;
            handler.write = sram::dma_write;
        }
        _ => {
            handler.read = no_device_dma_read;
            handler.write = no_device_dma_write;
        }
    }

    handler
}

fn no_device_dma_read(n64: &mut N64, cart_addr: u32, _dram_addr: u32, length: u32) -> u64 {
    calculate_cycles_for_transfer(n64, 1, cart_addr, length)
}

fn no_device_dma_write(n64: &mut N64, cart_addr: u32, dram_addr: u32, length: u32) -> u64 {
    for offset in 0..length {
        memory::write_rdram_byte(n64, dram_addr.wrapping_add(offset), 0);
    }
    calculate_cycles_for_transfer(n64, 1, cart_addr, length)
}

pub fn write_regs(n64: &mut N64, address: u64, value: u32, mask: u32) {
    let reg = register_index(address);
    if reg >= PI_REGS_COUNT as usize {
        return;
    }

    let status = n64.peripheral.regs[PiRegister::Status as usize];
    if reg != PiRegister::Status as usize && status & (PiStatus::DMA_BUSY | PiStatus::IO_BUSY) != 0
    {
        n64.peripheral.regs[PiRegister::Status as usize] |= PI_STATUS_ERROR;
        return;
    }

    match reg as u32 {
        x if x == PiRegister::RdLen as u32 => {
            memory::masked_write_32(&mut n64.peripheral.regs[reg], value, mask);
            dma_read(n64);
        }
        x if x == PiRegister::WrLen as u32 => {
            memory::masked_write_32(&mut n64.peripheral.regs[reg], value, mask);
            dma_write(n64);
        }
        x if x == PiRegister::Status as u32 => {
            if value & mask & PiStatus::CLR_INTR != 0 {
                n64.peripheral.regs[reg] &= !(PiStatus::INTERRUPT);
                mips::clear_rcp_interrupt(n64, mips::MI_INTR_PI);
            }

            if value & mask & PiStatus::RESET != 0 {
                if status & PiStatus::DMA_BUSY != 0 {
                    crate::events::remove_event(n64, EventType::PI);
                }
                n64.peripheral.regs[reg] &= !(PiStatus::DMA_BUSY | PI_STATUS_ERROR);
            }
        }
        x if (PiRegister::BsdDom1Lat as u32..=PiRegister::BsdDom2Rls as u32).contains(&x) => {
            let register_mask = match (x - PiRegister::BsdDom1Lat as u32) % DOMAIN_REGISTER_COUNT {
                DOMAIN_PAGE_SIZE_REGISTER => DOMAIN_PAGE_SIZE_MASK,
                DOMAIN_RELEASE_REGISTER => DOMAIN_RELEASE_MASK,
                _ => u8::MAX as u32,
            };

            memory::masked_write_32(&mut n64.peripheral.regs[reg], value & register_mask, mask);
        }
        _ => memory::masked_write_32(&mut n64.peripheral.regs[reg], value, mask),
    }
}

fn get_pi_domain(domain_id: i32) -> PiDomain {
    match domain_id {
        1 => PiDomain::Dom1,
        2 => PiDomain::Dom2,
        _ => panic!("Invalid PI DMA domain: {}", domain_id),
    }
}

pub fn calculate_cycles(n64: &N64, domain_id: i32, length: u32) -> u64 {
    let regs = DomainRegs::from_registers(&n64.peripheral.regs, get_pi_domain(domain_id));

    regs.cycles(length)
}

pub(crate) fn calculate_cycles_for_transfer(
    n64: &N64,
    domain_id: i32,
    cart_address: u32,
    length: u32,
) -> u64 {
    let regs = DomainRegs::from_registers(&n64.peripheral.regs, get_pi_domain(domain_id));
    regs.transfer_cycles(cart_address, length)
}

pub fn dma_event(n64: &mut N64) {
    n64.peripheral.regs[PiRegister::Status as usize] &= !(PiStatus::DMA_BUSY | PiStatus::IO_BUSY);
    n64.peripheral.regs[PiRegister::Status as usize] |= PiStatus::INTERRUPT;

    mips::set_rcp_interrupt(n64, mips::MI_INTR_PI);
}

#[cfg(test)]
mod tests;
