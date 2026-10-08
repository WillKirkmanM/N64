mod types;
pub use types::*;
mod joybus;
pub use joybus::process;

use crate::builder::memory::Memory;
use crate::N64;

use sha2::{Digest, Sha256};

use super::events::create_event;
use super::memory;
use super::mmu::add_cycles;
use super::peripheral;
use super::rdram::RDRAM_MASK;
use super::{events::EventType, mmu::MmuRegister, peripheral::{PiRegister, PiStatus}};

pub const CART_MASK: usize = 0xFFFFFFF;

pub const EEPROM_BLOCK_SIZE: usize = 8;

pub const EEPROM_MAX_SIZE: usize = 0x800;
const ROM_GAME_ID_START: usize = 0x3B;
const ROM_COUNTRY_OFFSET: usize = 0x3E;
const IPL3_START: usize = 0x40;
const IPL3_END: usize = 0x1000;
const RDRAM_SIZE_BOOT_OFFSET: usize = 0x318;
const CIC_6105_RDRAM_SIZE_BOOT_OFFSET: usize = 0x3F0;
const CART_UPPER_HALFWORD_MASK: u32 = 0xffff_0000;
const PAL_COUNTRY_CODES: &[u8] = b"DFIPSUXY";

pub fn read_mem_fast(n64: &mut N64, address: u64, _access_size: memory::AccessSize) -> u32 {
    n64.cartridge.rom.read_mem_fast(address)
}

fn read_rom_word(n64: &N64, address: u64) -> u32 {
    let offset = address as usize & CART_MASK;

    u32::from_be_bytes(std::array::from_fn(|byte| {
        n64.cartridge
            .rom
            .data
            .get(offset + byte)
            .copied()
            .unwrap_or(0)
    }))
}

pub fn read_mem(n64: &mut N64, address: u64, _access_size: memory::AccessSize) -> u32 {
    let cycles = peripheral::calculate_cycles(n64, 1, 4);
    add_cycles(n64, cycles);

    if n64.peripheral.regs[PiRegister::Status as usize] & PiStatus::IO_BUSY != 0 {
        n64.cartridge.latch
    } else {
        n64.cartridge.rom.read_mem_fast(address) as u32
    }
}

pub fn write_mem(n64: &mut N64, _address: u64, value: u32, _mask: u32) {
    if n64.peripheral.regs[PiRegister::Status as usize] & PiStatus::IO_BUSY != 0 {
        return;
    }
    // PI latches all driven data lines, including the shifted upper bytes
    // of SB/SH; byte-enable masks do not restrict this write latch.
    n64.cartridge.latch = value;
    n64.peripheral.regs[PiRegister::Status as usize] |= PiStatus::IO_BUSY;

    let cycles = peripheral::calculate_cycles(n64, 1, 4);
    create_event(
        n64,
        EventType::PI,
        n64.cp0_register(MmuRegister::Count) + cycles,
        peripheral::dma_event,
    );
}

fn dma_transfer(
    n64: &mut N64,
    mut cart_addr: u32,
    mut dram_addr: u32,
    length: u32,
    is_read: bool,
) -> u64 {
    dram_addr &= RDRAM_MASK as u32;
    cart_addr &= CART_MASK as u32;

    let mut i = dram_addr;
    let mut j = cart_addr;

    while i < dram_addr + length && j < n64.cartridge.rom.len() as u32 {
        if is_read {
            n64.cartridge.rom[j as usize] = n64.rdram.mem[i as usize ^ n64.byte_swap];
        } else {
            n64.rdram.mem[i as usize ^ n64.byte_swap] = n64.cartridge.rom[j as usize];
        }
        i += 1;
        j += 1;
    }

    while i < dram_addr + length {
        if !is_read {
            n64.rdram.mem[i as usize ^ n64.byte_swap] = 0;
        }
        i += 1;
    }

    peripheral::calculate_cycles(n64, 1, length)
}

pub fn dma_read(n64: &mut N64, cart_addr: u32, dram_addr: u32, length: u32) -> u64 {
    dma_transfer(n64, cart_addr, dram_addr, length, true)
}

pub fn dma_write(n64: &mut N64, cart_addr: u32, dram_addr: u32, length: u32) -> u64 {
    dma_transfer(n64, cart_addr, dram_addr, length, false)
}

pub fn write_cartridge_to_memory(n64: &mut N64, rom_file: Vec<u8>) {
    n64.cartridge.rom = Memory::new()
        .with_size(rom_file.len())
        .with_mask(CART_MASK)
        .build();

    n64.cartridge.rom.data = rom_file.into();

    set_system_region(n64, n64.cartridge.rom.data[ROM_COUNTRY_OFFSET]);
    set_cic(n64);

    n64.ui.game_info.hash = calculate_hash(&n64.cartridge.rom.data);

    n64.ui.game_info.id =
        String::from_utf8(n64.cartridge.rom.data[ROM_GAME_ID_START..ROM_COUNTRY_OFFSET].to_vec())
            .unwrap_or_else(|_| String::from("UNK"));
}

pub fn load_rom_save(n64: &mut N64) {
    n64.restore_rom_save();
}

pub fn set_system_region(n64: &mut N64, country: u8) {
    n64.cartridge.boot.pal = PAL_COUNTRY_CODES.contains(&country);
}

fn set_cic(n64: &mut N64) {
    let hash = calculate_hash(&n64.cartridge.rom[IPL3_START..IPL3_END]);
    let (cic_type, cic_seed, rdram_size_offset) = match hash.as_str() {
        "B99F06C4802C2377E31E388435955EF3E99C618A6D55D24699D828EB1075F1EB" => {
            (CicType::CicNus6101, 0x3F, RDRAM_SIZE_BOOT_OFFSET)
        }
        "61E88238552C356C23D19409FE5570EE6910419586BC6FC740F638F761ADC46E" => {
            (CicType::CicNus6102, 0x3F, RDRAM_SIZE_BOOT_OFFSET)
        }
        "BF3620D30817007091EBE9BDDD1B88C23B8A0052170B3309CDE5B6B4238E45E7" => {
            (CicType::CicNus6103, 0x78, RDRAM_SIZE_BOOT_OFFSET)
        }
        "04B7BC6717A9F0EB724CF927E74AD3876C381CBB280D841736FC5E55580B756B" => {
            (CicType::CicNus6105, 0x91, CIC_6105_RDRAM_SIZE_BOOT_OFFSET)
        }
        "36ADC40148AF56F0D78CD505EB6A90117D1FD6F11C6309E52ED36BC4C6BA340E" => {
            (CicType::CicNus6106, 0x85, RDRAM_SIZE_BOOT_OFFSET)
        }
        "53C0088FB777870D0AF32F0251E964030E2E8B72E830C26042FD191169508C05" => {
            (CicType::CicNus5167, 0xdd, RDRAM_SIZE_BOOT_OFFSET)
        }
        _ => {
            println!("unknown IPL3 {}", hash);

            (CicType::CicNus6102, 0x3F, RDRAM_SIZE_BOOT_OFFSET)
        }
    };

    n64.cartridge.boot.cic_type = cic_type;
    n64.cartridge.boot.cic_seed = cic_seed;
    n64.cartridge.boot.rdram_size_offset = rdram_size_offset;
}

fn calculate_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);

    format!("{:X}", hasher.finalize())
}
