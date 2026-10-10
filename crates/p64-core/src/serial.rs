use crate::builder::registers::register_index;
mod types;
pub use types::*;

use super::{
    events::{create_event, EventType},
    memory,
    mmu::MmuRegister,
    N64,
};
use crate::{mips, pif};

const WRITE_DMA_CYCLES: u64 = 5200;

pub fn read_regs(n64: &mut N64, address: u64, _access_size: memory::AccessSize) -> u32 {
    n64.serial
        .regs
        .as_ref()
        .get(register_index(address))
        .copied()
        .unwrap_or(0)
}

pub fn write_regs(n64: &mut N64, address: u64, value: u32, mask: u32) {
    let reg = register_index(address);
    if reg >= SerialRegister::RegsCount as usize {
        return;
    }

    match reg as u32 {
        x if x == SerialRegister::Status as u32 => {
            n64.serial.regs[reg] &= !SerialStatus::INTERRUPT.bits();

            mips::clear_rcp_interrupt(n64, mips::MI_INTR_SI)
        }
        x if x == SerialRegister::PifAddrRd64b as u32 => dma_read(n64),
        x if x == SerialRegister::PifAddrWr64b as u32 => dma_write(n64),
        _ => memory::masked_write_32(&mut n64.serial.regs[reg], value, mask),
    }
}

pub fn dma_event(n64: &mut N64) {
    if n64.serial.dma_dir == DmaDir::Write {
        pif::process_ram(n64);
    } else if n64.serial.dma_dir == DmaDir::Read {
        copy_pif_rdram(n64);
    } else {
        panic!("si dma unknown")
    }
    n64.serial.dma_dir = DmaDir::None;
    n64.serial.regs[SerialRegister::Status as usize] &=
        !(SerialStatus::DMA_BUSY.bits() | SerialStatus::IO_BUSY.bits());
    n64.serial.regs[SerialRegister::Status as usize] |= SerialStatus::INTERRUPT.bits();

    mips::set_rcp_interrupt(n64, mips::MI_INTR_SI)
}

fn dma_read(n64: &mut N64) {
    n64.serial.dma_dir = DmaDir::Read;
    let duration = pif::update_pif_ram(n64);
    n64.serial.regs[SerialRegister::Status as usize] |= SerialStatus::DMA_BUSY.bits();

    create_event(
        n64,
        EventType::SI,
        n64.cp0_register(MmuRegister::Count) + duration,
        dma_event,
    )
}

fn dma_write(n64: &mut N64) {
    n64.serial.dma_dir = DmaDir::Write;
    copy_pif_rdram(n64);
    n64.serial.regs[SerialRegister::Status as usize] |= SerialStatus::DMA_BUSY.bits();

    create_event(
        n64,
        EventType::SI,
        n64.cp0_register(MmuRegister::Count) + WRITE_DMA_CYCLES,
        dma_event,
    )
}

fn copy_pif_rdram(n64: &mut N64) {
    let address = n64.serial.regs[SerialRegister::DramAddr as usize];
    for offset in 0..pif::PIF_RAM_SIZE {
        let dram_address = address.wrapping_add(offset as u32);
        match n64.serial.dma_dir {
            DmaDir::Write => {
                n64.pif.memory.ram[offset] = memory::read_rdram_byte(n64, dram_address)
            }
            DmaDir::Read => memory::write_rdram_byte(n64, dram_address, n64.pif.memory.ram[offset]),
            DmaDir::None => return,
        }
    }
}

#[cfg(test)]
mod tests;
