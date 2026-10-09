use p64_gui::storage::SaveTypes;

use super::{events, memory, mmu, peripheral, N64};

pub const FLASHRAM_TYPE_ID: u32 = 0x11118001;
pub const MX29L1100_ID: u32 = 0x00c2001e;

const SRAM_WINDOW_MASK: usize = 0xFFFF;
const SRAM_SIZE: usize = 0x8000;
const FLASHRAM_SIZE: usize = 0x20000;
const FLASHRAM_ADDRESS_MASK: u32 = (FLASHRAM_SIZE - 1) as u32;
const FLASHRAM_PAGE_BYTES: usize = 128;
const FLASHRAM_SECTOR_BYTES: usize = 128 * FLASHRAM_PAGE_BYTES;
const FLASHRAM_PAGE_MASK: usize = FLASHRAM_SIZE / FLASHRAM_PAGE_BYTES - 1;
const FLASHRAM_COMMAND_ADDRESS: u32 = 0x10000;
const FLASHRAM_ARRAY_WINDOW_BYTES: u32 = 0x1_0000;
const FLASHRAM_ARRAY_ADDRESS_MASK: u32 = FLASHRAM_ARRAY_WINDOW_BYTES - 1;
const FLASHRAM_COMMAND_MASK: u32 = 0xff00_0000;
const STATUS_PROGRAM_BUSY: u32 = 1;
const STATUS_ERASE_BUSY: u32 = 2;
const STATUS_PROGRAM_COMPLETE: u32 = 4;
const STATUS_ERASE_COMPLETE: u32 = 8;
const MX29L0000_ID: u32 = 0x00c20000;
const MX29L0001_ID: u32 = 0x00c20001;

mod types;
pub use types::{Flashram, FlashramMode};

#[repr(u32)]
enum FlashCommand {
    ChipErase = 0x3c00_0000,
    SectorErase = 0x4b00_0000,
    ExecuteErase = 0x7800_0000,
    ProgramPage = 0xa500_0000,
    LoadPage = 0xb400_0000,
    ReadStatus = 0xd200_0000,
    ReadSiliconId = 0xe100_0000,
    ReadArray = 0xf000_0000,
}

fn format_memory(memory: &mut Vec<u8>, size: usize) {
    if memory.len() < size {
        memory.resize(size, 0xFF);
    }
}

fn read_sram_byte(n64: &N64, offset: usize) -> u8 {
    if offset < SRAM_SIZE {
        n64.save_data().sram.data.get(offset).copied().unwrap_or(0)
    } else {
        0
    }
}

fn write_sram_byte(n64: &mut N64, offset: usize, value: u8, mask: u8) {
    if offset >= SRAM_SIZE {
        return;
    }
    let save = &mut n64.save_data_mut().sram;
    if let Some(destination) = save.data.get_mut(offset) {
        *destination = crate::bits::replace_bits(*destination, value, mask);
        save.dirty = true;
    }
}

pub fn dma_read(n64: &mut N64, cart_addr: u32, dram_addr: u32, length: u32) -> u64 {
    if n64.has_save_type(SaveTypes::Sram) {
        format_memory(&mut n64.save_data_mut().sram.data, SRAM_SIZE);
        let cart_addr = cart_addr as usize & SRAM_WINDOW_MASK;
        for offset in 0..length {
            let value = memory::read_rdram_byte(n64, dram_addr.wrapping_add(offset));
            write_sram_byte(n64, cart_addr + offset as usize, value, u8::MAX);
        }
    } else if cart_addr & FLASHRAM_ADDRESS_MASK == 0
        && length == FLASHRAM_PAGE_BYTES as u32
        && n64.flashram.mode == FlashramMode::PageProgram
    {
        for offset in 0..length {
            n64.flashram.page_buf[offset as usize] =
                memory::read_rdram_byte(n64, dram_addr.wrapping_add(offset));
        }
    }
    peripheral::calculate_cycles_for_transfer(n64, 2, cart_addr, length)
}

pub fn dma_write(n64: &mut N64, cart_addr: u32, dram_addr: u32, length: u32) -> u64 {
    if n64.has_save_type(SaveTypes::Sram) {
        format_memory(&mut n64.save_data_mut().sram.data, SRAM_SIZE);
        let cart_addr = cart_addr as usize & SRAM_WINDOW_MASK;
        for offset in 0..length {
            let value = read_sram_byte(n64, cart_addr + offset as usize);
            memory::write_rdram_byte(n64, dram_addr.wrapping_add(offset), value);
        }
    } else {
        dma_write_flash(n64, cart_addr, dram_addr, length);
    }
    peripheral::calculate_cycles_for_transfer(n64, 2, cart_addr, length)
}

fn dma_write_flash(n64: &mut N64, cart_addr: u32, dram_addr: u32, length: u32) {
    let address = cart_addr & FLASHRAM_ADDRESS_MASK;
    if address == 0 && length == 8 && n64.flashram.mode == FlashramMode::ReadSiliconId {
        let bytes = n64.flashram.silicon_id.map(u32::to_be_bytes).concat();
        for (offset, value) in bytes.into_iter().enumerate() {
            memory::write_rdram_byte(n64, dram_addr.wrapping_add(offset as u32), value);
        }
    } else if address < FLASHRAM_ARRAY_WINDOW_BYTES && n64.flashram.mode == FlashramMode::ReadArray
    {
        format_memory(&mut n64.save_data_mut().flash.data, FLASHRAM_SIZE);
        let address_scale = if matches!(
            n64.flashram.silicon_id[1],
            MX29L1100_ID | MX29L0000_ID | MX29L0001_ID
        ) {
            2
        } else {
            1
        };
        let address = (cart_addr & FLASHRAM_ARRAY_ADDRESS_MASK) as usize * address_scale;
        for offset in 0..length {
            let value = n64
                .save_data()
                .flash
                .data
                .get(address + offset as usize)
                .copied()
                .unwrap_or(0);
            memory::write_rdram_byte(n64, dram_addr.wrapping_add(offset), value);
        }
    }
}
