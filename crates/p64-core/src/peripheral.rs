use p64_gui::input;

use crate::{cartridge, mips, sram};
use super::{
    builder::registers::Registers,
    mmu::MmuRegister,
    events::{create_event, EventType},
    memory::{self, MemoryMap},
    N64,
};

pub const PI_REGS_COUNT: u32 = 13;

pub fn read_regs(n64: &mut N64, address: u64, _access_size: memory::AccessSize) -> u32 {
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        x if x == PiRegister::WrLen as u32 || x == PiRegister::RdLen as u32 => 0x7F,
        x if x == PiRegister::CartAddr as u32 => n64.peripheral.regs[reg as usize] & 0xFFFFFFFE,
        x if x == PiRegister::DramAddr as u32 => n64.peripheral.regs[reg as usize] & 0xFFFFFE,
        _ => n64.peripheral.regs[reg as usize],
    }
}

fn dma_transfer(n64: &mut N64, is_read: bool) {
    let handler = get_handler(n64.peripheral.regs[PiRegister::CartAddr as usize]);

    let cart_addr = n64.peripheral.regs[PiRegister::CartAddr as usize] & !1;
    let dram_addr = n64.peripheral.regs[PiRegister::DramAddr as usize] & 0xFFFFFE;
    let mut length = (n64.peripheral.regs[if is_read {
        PiRegister::RdLen
    } else {
        PiRegister::WrLen
    } as usize]
        & 0xFFFFFF)
        + 1;

    if length >= 0x7f && (length & 1) != 0 {
        length += 1;
    }

    let cycles = if is_read {
        (handler.read)(n64, cart_addr, dram_addr, length)
    } else {
        (handler.write)(n64, cart_addr, dram_addr, length)
    };

    create_event(
        n64,
        EventType::PI,
        n64.cpu.mmu.regs[MmuRegister::Count as usize] + cycles,
        dma_event,
    );

    n64.peripheral.regs[PiRegister::DramAddr as usize] =
        (n64.peripheral.regs[PiRegister::DramAddr as usize] + length + 7) & !7;
    n64.peripheral.regs[PiRegister::CartAddr as usize] =
        (n64.peripheral.regs[PiRegister::CartAddr as usize] + length + 1) & !1;

    n64.peripheral.regs[PiRegister::Status as usize] |= PiStatus::DMA_BUSY as u32;
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
        _ => panic!("unknown pi handler"),
    }

    handler
}

pub fn write_regs(n64: &mut N64, address: u64, value: u32, mask: u32) {
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        x if x == PiRegister::RdLen as u32 => {
            memory::masked_write_32(&mut n64.peripheral.regs[reg as usize], value, mask);
            dma_read(n64);
        }
        x if x == PiRegister::WrLen as u32 => {
            memory::masked_write_32(&mut n64.peripheral.regs[reg as usize], value, mask);
            dma_write(n64);
        }
        x if x == PiRegister::Status as u32 => {
            if value & mask & PiStatus::CLR_INTR as u32 != 0 {
                n64.peripheral.regs[reg as usize] &= !(PiStatus::INTERRUPT as u32);
                mips::clear_rcp_interrupt(n64, mips::MI_INTR_PI);
            }
            if value & mask & PiStatus::RESET as u32 != 0 {
                n64.peripheral.regs[PiRegister::Status as usize] = 0;
            }
        }
        x if (PiRegister::BsdDom1Lat as u32..=PiRegister::BsdDom2Rls as u32).contains(&x) => {
            memory::masked_write_32(&mut n64.peripheral.regs[reg as usize], value & 0xFF, mask);
        }
        _ => memory::masked_write_32(&mut n64.peripheral.regs[reg as usize], value, mask),
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
    let regs = DomainRegs::from_device(n64, get_pi_domain(domain_id));

    let page_size = 2.0_f64.powf((regs.page_size + PAGE_SIZE_OFFSET) as f64);
    let total_pages = (length as f64 / page_size).ceil();

    let latency_cycles = (BASE_CYCLES + (regs.latency + 1) as f64) * total_pages;
    let transfer_cycles =
        ((regs.pulse_width + 1) as f64 + (regs.release + 1) as f64) * (length as f64 / 2.0);
    let overhead_cycles = PAGE_CYCLES * total_pages;

    ((latency_cycles + transfer_cycles + overhead_cycles) * RCP_TO_CPU_CLOCK_RATIO) as u64
}

pub fn dma_event(n64: &mut N64) {
    n64.peripheral.regs[PiRegister::Status as usize] &=
        !(PiStatus::DMA_BUSY as u32 | PiStatus::IO_BUSY as u32);
    n64.peripheral.regs[PiRegister::Status as usize] |= PiStatus::INTERRUPT as u32;

    mips::set_rcp_interrupt(n64, mips::MI_INTR_PI);
}

const CONTROLLER_FLAGS: u16 = 0x0005; // JDT_JOY_ABS_COUNTERS | JDT_JOY_PORT
const PAK_CHUNK_SIZE: usize = 0x20;

pub fn process(n64: &mut N64, channel: usize) {
    let cmd = n64.pif.ram[n64.pif.channels[channel].tx_buf.unwrap()];
    let cmd = unsafe { std::mem::transmute(cmd) };

    match cmd {
        JoyCommand::Status | JoyCommand::Reset => handle_status(n64, channel),
        JoyCommand::ControllerRead => handle_read(n64, channel),
        JoyCommand::PakRead => handle_pak_read(n64, channel),
        JoyCommand::PakWrite => handle_pak_write(n64, channel),
    }
}

fn handle_status(n64: &mut N64, channel: usize) {
    let rx_buf = n64.pif.channels[channel].rx_buf.unwrap();
    n64.pif.ram[rx_buf..rx_buf + 2].copy_from_slice(&CONTROLLER_FLAGS.to_ne_bytes());
    n64.pif.ram[rx_buf + 2] = if n64.pif.channels[channel].pak_handler.is_some() {
        1
    } else {
        0
    };
}

fn handle_read(n64: &mut N64, channel: usize) {
    let offset = n64.pif.channels[channel].rx_buf.unwrap();
    let input = input::get(&mut n64.ui, channel);
    n64.pif.ram[offset..offset + 4].copy_from_slice(&input.to_ne_bytes());
}

fn handle_pak_read(n64: &mut N64, channel: usize) {
    let (addr_offset, data_offset) = get_pak_offsets(n64, channel);
    let address = get_pak_address(n64, addr_offset);

    if let Some(handler) = n64.pif.channels[channel].pak_handler {
        (handler.read)(n64, channel, address, data_offset, PAK_CHUNK_SIZE);
        n64.pif.ram[data_offset + PAK_CHUNK_SIZE] = calculate_crc(n64, data_offset);
    }
}

fn handle_pak_write(n64: &mut N64, channel: usize) {
    let (addr_offset, data_offset) = get_pak_offsets(n64, channel);
    let address = get_pak_address(n64, addr_offset);

    if data_offset + PAK_CHUNK_SIZE >= n64.pif.ram.len() {
        return;
    }

    if let Some(handler) = n64.pif.channels[channel].pak_handler {
        (handler.write)(n64, channel, address, data_offset, PAK_CHUNK_SIZE);
        // Only write CRC if we have space
        if data_offset + PAK_CHUNK_SIZE < n64.pif.ram.len() {
            n64.pif.ram[data_offset + PAK_CHUNK_SIZE] = calculate_crc(n64, data_offset);
        }
    }
}

#[inline]
fn get_pak_address(n64: &N64, addr_offset: usize) -> u16 {
    (n64.pif.ram[addr_offset] as u16) << 8 | (n64.pif.ram[addr_offset + 1] & 0xe0) as u16
}

#[inline]
fn get_pak_offsets(n64: &N64, channel: usize) -> (usize, usize) {
    let tx = n64.pif.channels[channel].tx_buf.unwrap();
    let rx = n64.pif.channels[channel].rx_buf.unwrap();
    (tx + 1, rx)
}

fn calculate_crc(n64: &N64, data_offset: usize) -> u8 {
    // Early return if starting offset is already out of bounds
    if data_offset >= n64.pif.ram.len() {
        return 0;
    }

    // Calculate how many bytes we can safely read
    let available_bytes = n64.pif.ram.len() - data_offset;
    let chunk_size = std::cmp::min(PAK_CHUNK_SIZE, available_bytes);

    (0..chunk_size).fold(0u8, |crc, i| {
        (0..8).fold(crc, |crc, bit| {
            let xor_tap = if crc & 0x80 != 0 { 0x85 } else { 0x00 };
            let mut crc = crc << 1;

            if n64.pif.ram[data_offset + i] & (0x80 >> bit) != 0 {
                crc |= 1;
            }
            crc ^ xor_tap
        })
    })
}

mod types;
pub use types::{PiRegister, PiStatus, Peripheral, PiHandler, PakHandler};
use types::{
    RCP_TO_CPU_CLOCK_RATIO, BASE_CYCLES, PAGE_CYCLES, PAGE_SIZE_OFFSET, PiDomain, DomainRegs,
    JoyCommand,
};
