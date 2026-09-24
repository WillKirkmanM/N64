use crate::serial;

// use crate::{n64, serial};
use super::{
    cartridge,
    mmu::{add_cycles, MmuRegister},
    events::{create_event, EventType},
    memory::{self, masked_write_32},
    mempak,
    peripheral::{self, PakHandler},
    serial::{DmaDir, SerialRegister, SerialStatus},
    N64,
};

/// PIF Memory Layout
pub const PIF_RAM_SIZE: usize = 64;
const PIF_RAM_OFFSET: usize = 0x7C0;
const PIF_MASK: usize = 0xFFFF;
const PIF_CHANNELS_COUNT: usize = 5;

/// PIF RAM Command Values
const PIF_RAM_COMMAND_ADDR: usize = 0x3F;
const PIF_CIC_SEED_ADDR1: usize = 0x26;
const PIF_CIC_SEED_ADDR2: usize = 0x27;

/// Channel Configuration
const CONTROLLER_CHANNEL: usize = 0;
const CART_CHANNEL: usize = 4;

/// RAM Control Bytes
const RAM_EMPTY: u8 = 0x00;
const RAM_TERMINATOR: u8 = 0xFE;
const RAM_SKIP: u8 = 0xFF;
const RAM_DISABLE: u8 = 0xFD;

/// CIC Challenge Parameters
const CIC_CHALLENGE_SIZE: usize = 15;
const CIC_RAM_OFFSET: usize = 0x30;
const CIC_RESPONSE_LENGTH: usize = 28;

/// CIC Challenge Lookup Tables
const LUT0: [u8; 16] = [
    0x4, 0x7, 0xA, 0x7, 0xE, 0x5, 0xE, 0x1, 0xC, 0xF, 0x8, 0xF, 0x6, 0x3, 0x6, 0x9,
];
const LUT1: [u8; 16] = [
    0x4, 0x1, 0xA, 0x7, 0xE, 0x5, 0xE, 0x1, 0xC, 0x9, 0x8, 0x5, 0x6, 0x3, 0xC, 0x9,
];

pub fn read_mem(n64: &mut N64, address: u64, _access_size: memory::AccessSize) -> u32 {
    add_cycles(n64, 3000);

    let masked_address = address as usize & PIF_MASK;
    if masked_address < PIF_RAM_OFFSET {
        u32::from_be_bytes(
            n64.pif.rom[masked_address..masked_address + 4]
                .try_into()
                .unwrap(),
        )
    } else {
        let ram_address = masked_address - PIF_RAM_OFFSET;
        u32::from_be_bytes(
            n64.pif.ram[ram_address..ram_address + 4]
                .try_into()
                .unwrap(),
        )
    }
}

pub fn write_mem(n64: &mut N64, address: u64, value: u32, mask: u32) {
    let masked_address = address as usize & PIF_MASK;

    // Hack so n64-systemtest runs.
    if masked_address < PIF_RAM_OFFSET {
        return;
    }

    let ram_address = masked_address - PIF_RAM_OFFSET;
    let mut data = u32::from_be_bytes(
        n64.pif.ram[ram_address..ram_address + 4]
            .try_into()
            .unwrap(),
    );
    masked_write_32(&mut data, value, mask);
    n64.pif.ram[ram_address..ram_address + 4].copy_from_slice(&data.to_be_bytes());

    n64.serial.dma_dir = DmaDir::Write;
    create_event(
        n64,
        EventType::SI,
        n64.cpu.mmu.regs[MmuRegister::Count as usize] + 3200,
        serial::dma_event,
    );
    n64.serial.regs[SerialRegister::Status as usize] |=
        SerialStatus::DMA_BUSY.bits() | SerialStatus::IO_BUSY.bits();
}

fn process_channel(n64: &mut N64, channel: usize) -> usize {
    let pif_channel = &mut n64.pif.channels[channel];
    if pif_channel.tx.is_none() {
        return 0;
    }

    n64.pif.ram[pif_channel.tx.unwrap()] &= 0x3f;
    n64.pif.ram[pif_channel.rx.unwrap()] &= 0x3f;

    if pif_channel.process.is_none() {
        n64.pif.ram[pif_channel.rx.unwrap()] |= 0x80;
        return 0;
    }

    (pif_channel.process.unwrap())(n64, channel);
    1
}

pub fn update_pif_ram(n64: &mut N64) -> u64 {
    let active_channels = (0..PIF_CHANNELS_COUNT)
        .map(|k| process_channel(n64, k))
        .sum::<usize>();
    (24000 + (active_channels * 28000)) as u64
}

fn disable_pif_channel(channel: &mut PifChannel) {
    channel.tx = None;
    channel.rx = None;
    channel.tx_buf = None;
    channel.rx_buf = None;
}

fn setup_pif_channel(n64: &mut N64, channel: usize, buf: usize) -> usize {
    let tx = n64.pif.ram[buf] & 0x3f;
    let rx = n64.pif.ram[buf + 1] & 0x3f;

    n64.pif.channels[channel].tx = Some(buf);
    n64.pif.channels[channel].rx = Some(buf + 1);
    n64.pif.channels[channel].tx_buf = Some(buf + 2);
    n64.pif.channels[channel].rx_buf = Some(buf + 2 + tx as usize);

    (2 + tx + rx) as usize
}

fn setup_channels_format(n64: &mut N64) {
    let mut ram_idx = 0;
    let mut channel_idx = 0;

    while ram_idx < PIF_RAM_SIZE && channel_idx < PIF_CHANNELS_COUNT {
        match n64.pif.ram[ram_idx] {
            RAM_EMPTY => {
                disable_pif_channel(&mut n64.pif.channels[channel_idx]);
                channel_idx += 1;
                ram_idx += 1;
            }
            RAM_SKIP => ram_idx += 1,
            RAM_TERMINATOR => {
                n64.pif.channels[channel_idx..PIF_CHANNELS_COUNT]
                    .iter_mut()
                    .for_each(disable_pif_channel);
                break;
            }
            RAM_DISABLE => {
                disable_pif_channel(&mut n64.pif.channels[channel_idx]);
                channel_idx += 1;
                ram_idx += 1;
            }
            _ => {
                // Check for early termination sequence
                if ram_idx + 1 < PIF_RAM_SIZE && n64.pif.ram[ram_idx + 1] == RAM_TERMINATOR {
                    ram_idx += 1;
                    continue;
                }

                // Check for buffer overflow
                if ram_idx + 2 >= PIF_RAM_SIZE {
                    break;
                }

                ram_idx += setup_pif_channel(n64, channel_idx, ram_idx);
                channel_idx += 1;
            }
        }
    }
}

pub fn process_ram(n64: &mut N64) {
    let mut clrmask = 0;
    let command = n64.pif.ram[PIF_RAM_COMMAND_ADDR];

    // Process commands using bit patterns
    [
        (PifCommand::SetupChannels, 0x01),
        (PifCommand::ProcessChallenge, 0x02),
        (PifCommand::Unknown, 0x08),
        (PifCommand::ClearRom, 0x10),
        (PifCommand::SetStatusBit, 0x20),
    ]
    .into_iter()
    .for_each(|(cmd, mask)| {
        if command & mask != 0 {
            match cmd {
                PifCommand::SetupChannels => {
                    setup_channels_format(n64);
                    clrmask |= mask;
                }
                PifCommand::ProcessChallenge => {
                    n64.pif.channels[..PIF_CHANNELS_COUNT]
                        .iter_mut()
                        .for_each(disable_pif_channel);
                    process_cic_challenge(n64);
                    clrmask |= mask;
                }
                PifCommand::Unknown => {
                    clrmask |= mask;
                }
                PifCommand::ClearRom => {
                    n64.pif.rom.fill(0);
                }
                PifCommand::SetStatusBit => {
                    n64.pif.ram[PIF_RAM_COMMAND_ADDR] = 0x80;
                }
            }
        }
    });

    n64.pif.ram[PIF_RAM_COMMAND_ADDR] &= !clrmask;
}

pub fn set_cartridge_region(n64: &mut N64) {
    n64.pif.rom = match n64.cartridge.pal {
        true => PAL_PIF_ROM,
        false => NTSC_PIF_ROM,
    };

    [PIF_CIC_SEED_ADDR1, PIF_CIC_SEED_ADDR2]
        .iter()
        .for_each(|&addr| n64.pif.ram[addr] = n64.cartridge.cic_seed);
}

fn process_cic_challenge(n64: &mut N64) {
    let mut challenge = [0u8; 30];
    let mut response = [0u8; 30];

    // Pack bytes into nibbles
    challenge
        .chunks_mut(2)
        .zip(n64.pif.ram[CIC_RAM_OFFSET..CIC_RAM_OFFSET + CIC_CHALLENGE_SIZE].iter())
        .for_each(|(chunk, &byte)| {
            chunk[0] = (byte >> 4) & 0xf;
            chunk[1] = byte & 0xf;
        });

    n64_cic_nus_6105(challenge, &mut response, CIC_RESPONSE_LENGTH);

    // Clear status bytes
    n64.pif.ram[0x2e..0x30].fill(0);

    // Unpack nibbles back into bytes
    n64.pif.ram[CIC_RAM_OFFSET..CIC_RAM_OFFSET + CIC_CHALLENGE_SIZE]
        .iter_mut()
        .zip(response.chunks(2))
        .for_each(|(byte, nibbles)| *byte = (nibbles[0] << 4) | nibbles[1]);
}

fn n64_cic_nus_6105(challenge: [u8; 30], response: &mut [u8; 30], len: usize) {
    let mut key = 0xB;
    let mut current_lut = &LUT0;

    for i in 0..len {
        // Calculate response value
        response[i] = (key + 5 * challenge[i]) & 0xF;
        key = current_lut[response[i] as usize];

        // Calculate sign and magnitude
        let sign = (response[i] >> 3) & 0x1;
        let magnitude = if sign == 1 { !response[i] } else { response[i] } & 0x7;
        let mod_value = if magnitude % 3 == 1 { sign } else { 1 - sign };

        // Determine which lookup table to use next
        current_lut = match (current_lut, response[i]) {
            (&LUT1, 0x1 | 0x9) => &LUT1,
            (&LUT1, 0xB | 0xE) => &LUT0,
            _ if mod_value == 1 => &LUT1,
            _ => &LUT0,
        };
    }
}

mod rom;
pub use rom::{NTSC_PIF_ROM, PAL_PIF_ROM};

mod types;
pub use types::{Pif, PifChannel};
use types::{PifCommand};
