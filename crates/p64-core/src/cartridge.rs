use chrono::Datelike;
use chrono::Timelike;

use crate::N64;
use p64_gui::storage::SaveTypes;

use sha2::{Digest, Sha256};

use super::mmu::add_cycles;
use super::events::create_event;
use super::memory;
use super::peripheral;
use super::rdram::RDRAM_MASK;
use super::{
    builder::memory::Memory,
    mmu::MmuRegister,
    events::EventType,
    peripheral::{PiRegister, PiStatus},
};

pub const CART_MASK: usize = 0xFFFFFFF;

pub const EEPROM_BLOCK_SIZE: usize = 8;
pub const EEPROM_MAX_SIZE: usize = 0x800;

#[derive(Clone, Default)]
pub struct AfRtc {
    pub control: u16,
}

pub fn read_mem_fast(n64: &mut N64, address: u64, _access_size: memory::AccessSize) -> u32 {
    n64.cartridge.rom.read_mem_fast(address)
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

pub fn write_mem(n64: &mut N64, _address: u64, value: u32, mask: u32) {
    n64.cartridge.latch = value & mask;
    n64.peripheral.regs[PiRegister::Status as usize] |= PiStatus::IO_BUSY;

    let cycles = peripheral::calculate_cycles(n64, 1, 4);
    create_event(
        n64,
        EventType::PI,
        n64.cpu.mmu.regs[MmuRegister::Count as usize] + cycles,
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

    n64.cartridge.rom.data = rom_file;

    set_system_region(n64, n64.cartridge.rom.data[0x3E]);
    set_cic(n64);

    n64.ui.game_info.hash = calculate_hash(&n64.cartridge.rom.data);

    n64.ui.game_info.id = String::from_utf8(n64.cartridge.rom.data[0x3B..0x3E].to_vec())
        .unwrap_or_else(|_| String::from("UNK"));
}

pub fn load_rom_save(n64: &mut N64) {
    if n64.ui.save_data.data.romsave.0.is_empty() {
        return;
    }
    for (key, value) in n64.ui.save_data.data.romsave.0.iter() {
        n64.cartridge.rom[key.parse::<usize>().unwrap()] =
            serde_json::from_value(value.clone()).unwrap()
    }
}

pub fn set_system_region(n64: &mut N64, country: u8) {
    let pal_codes: [u8; 8] = [b'D', b'F', b'I', b'P', b'S', b'U', b'X', b'Y'];
    n64.cartridge.pal = pal_codes.contains(&country);
}

fn set_cic(n64: &mut N64) {
    let hash = calculate_hash(&n64.cartridge.rom[0x40..0x1000]);
    let (cic_type, cic_seed, rdram_size_offset) = match hash.as_str() {
        "B99F06C4802C2377E31E388435955EF3E99C618A6D55D24699D828EB1075F1EB" => {
            (CicType::CicNus6101, 0x3F, 0x318)
        }
        "61E88238552C356C23D19409FE5570EE6910419586BC6FC740F638F761ADC46E" => {
            (CicType::CicNus6102, 0x3F, 0x318)
        }
        "BF3620D30817007091EBE9BDDD1B88C23B8A0052170B3309CDE5B6B4238E45E7" => {
            (CicType::CicNus6103, 0x78, 0x318)
        }
        "04B7BC6717A9F0EB724CF927E74AD3876C381CBB280D841736FC5E55580B756B" => {
            (CicType::CicNus6105, 0x91, 0x3F0)
        }
        "36ADC40148AF56F0D78CD505EB6A90117D1FD6F11C6309E52ED36BC4C6BA340E" => {
            (CicType::CicNus6106, 0x85, 0x318)
        }
        "53C0088FB777870D0AF32F0251E964030E2E8B72E830C26042FD191169508C05" => {
            (CicType::CicNus5167, 0xdd, 0x318)
        }
        _ => {
            println!("unknown IPL3 {}", hash);
            (CicType::CicNus6102, 0x3F, 0x318)
        }
    };

    n64.cartridge.cic_type = cic_type;
    n64.cartridge.cic_seed = cic_seed;
    n64.cartridge.rdram_size_offset = rdram_size_offset;
}

fn calculate_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:X}", hasher.finalize())
}

fn byte2bcd(mut n: u32) -> u8 {
    n %= 100;
    (((n / 10) << 4) | (n % 10)) as u8
}

fn time2data(n64: &mut N64, offset: usize) {
    let now: chrono::DateTime<chrono::Local> = chrono::Local::now();

    n64.pif.ram[offset] = byte2bcd(now.second());
    n64.pif.ram[offset + 1] = byte2bcd(now.minute());
    n64.pif.ram[offset + 2] = 0x80 + byte2bcd(now.hour());
    n64.pif.ram[offset + 3] = byte2bcd(now.day());
    n64.pif.ram[offset + 4] = byte2bcd(now.weekday().num_days_from_sunday());
    n64.pif.ram[offset + 5] = byte2bcd(now.month());
    n64.pif.ram[offset + 6] = byte2bcd(now.year() as u32 - 1900);
    n64.pif.ram[offset + 7] = byte2bcd((now.year() as u32 - 1900) / 100);
}

mod joybus;
pub use joybus::{process};
use joybus::{
    handle_status_or_reset, handle_eeprom_read, handle_eeprom_write, handle_af_rtc_status,
    handle_af_rtc_read, handle_af_rtc_write, format_eeprom, eeprom_read_block, eeprom_write_block,
    af_rtc_read_block, af_rtc_write_block,
};

mod types;
pub use types::{CicType, Cartridge, JCmd, Jdt};
