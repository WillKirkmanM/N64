use crate::serial;

use super::{
    events::{create_event, EventType},
    memory::{self},
    mmu::{add_cycles, MmuRegister},
    serial::{DmaDir, SerialRegister, SerialStatus},
    N64,
};

pub const PIF_RAM_SIZE: usize = 64;
const PIF_RAM_OFFSET: usize = 0x7C0;
const PIF_MASK: usize = 0x7FF;
const PIF_CHANNELS_COUNT: usize = 5;
const MEMORY_READ_CYCLES: u64 = 3000;
const MEMORY_WRITE_CYCLES: u64 = 3200;
const TRANSFER_SETUP_CYCLES: usize = 24_000;
const CHANNEL_TRANSFER_CYCLES: usize = 28_000;
const CHANNEL_LENGTH_MASK: u8 = 0x3f;
const CHANNEL_NO_DEVICE: u8 = 0x80;
const STATUS_READY: u8 = 0x80;

const PIF_RAM_COMMAND_ADDR: usize = 0x3F;
const PIF_CIC_SEED_ADDR1: usize = 0x26;
const PIF_CIC_SEED_ADDR2: usize = 0x27;

const CONTROLLER_CHANNEL: usize = 0;
pub(crate) const CART_CHANNEL: usize = 4;

const RAM_EMPTY: u8 = 0x00;
const RAM_TERMINATOR: u8 = 0xFE;
const RAM_SKIP: u8 = 0xFF;
const RAM_DISABLE: u8 = 0xFD;

const CIC_CHALLENGE_SIZE: usize = 15;
const CIC_RAM_OFFSET: usize = 0x30;
const CIC_RESPONSE_LENGTH: usize = 28;
const CIC_NIBBLE_COUNT: usize = CIC_CHALLENGE_SIZE * 2;
const CIC_STATUS_OFFSET: usize = 0x2e;

const LUT0: [u8; 16] = [
    0x4, 0x7, 0xA, 0x7, 0xE, 0x5, 0xE, 0x1, 0xC, 0xF, 0x8, 0xF, 0x6, 0x3, 0x6, 0x9,
];
const LUT1: [u8; 16] = [
    0x4, 0x1, 0xA, 0x7, 0xE, 0x5, 0xE, 0x1, 0xC, 0x9, 0x8, 0x5, 0x6, 0x3, 0xC, 0x9,
];

mod rom;
pub use rom::{NTSC_PIF_ROM, PAL_PIF_ROM};
mod types;
use types::PifCommand;
pub use types::{Pif, PifChannel};

pub fn read_mem(n64: &mut N64, address: u64, _access_size: memory::AccessSize) -> u32 {
    add_cycles(n64, MEMORY_READ_CYCLES);

    let masked_address = address as usize & PIF_MASK;

    let bytes = if masked_address < PIF_RAM_OFFSET {
        n64.pif.memory.rom.get(masked_address..masked_address + 4)
    } else {
        let offset = masked_address - PIF_RAM_OFFSET;
        n64.pif.memory.ram.get(offset..offset + 4)
    };
    bytes
        .map(|bytes| u32::from_be_bytes(bytes.try_into().unwrap()))
        .unwrap_or(0)
}

pub fn write_mem(n64: &mut N64, address: u64, value: u32, _mask: u32) {
    let masked_address = address as usize & PIF_MASK;

    if masked_address < PIF_RAM_OFFSET {
        return;
    }

    let ram_address = masked_address - PIF_RAM_OFFSET;
    // PIF's bus ignores byte enables: SB/SH overwrite the entire word with
    // the CPU's shifted 32-bit bus value.
    let Some(destination) = n64.pif.memory.ram.get_mut(ram_address..ram_address + 4) else {
        return;
    };
    destination.copy_from_slice(&value.to_be_bytes());

    n64.serial.dma_dir = DmaDir::Write;
    create_event(
        n64,
        EventType::SI,
        n64.cp0_register(MmuRegister::Count) + MEMORY_WRITE_CYCLES,
        serial::dma_event,
    );
    n64.serial.regs[SerialRegister::Status as usize] |=
        SerialStatus::DMA_BUSY.bits() | SerialStatus::IO_BUSY.bits();
}

fn process_channel(n64: &mut N64, channel: usize) -> usize {
    if !validate_command(n64, channel, 1, 0) {
        return 0;
    }
    let pif_channel = &mut n64.pif.channels[channel];
    let Some(tx) = pif_channel.buffers.tx else {
        return 0;
    };

    let rx = pif_channel
        .buffers
        .rx
        .expect("configured PIF receive length");
    n64.pif.memory.ram[tx] &= CHANNEL_LENGTH_MASK;
    n64.pif.memory.ram[rx] &= CHANNEL_LENGTH_MASK;

    let Some(process) = pif_channel.process else {
        n64.pif.memory.ram[rx] |= CHANNEL_NO_DEVICE;

        return 0;
    };

    process(n64, channel);

    1
}

pub fn update_pif_ram(n64: &mut N64) -> u64 {
    let active_channels = (0..PIF_CHANNELS_COUNT)
        .map(|k| process_channel(n64, k))
        .sum::<usize>();

    (TRANSFER_SETUP_CYCLES + (active_channels * CHANNEL_TRANSFER_CYCLES)) as u64
}

pub(crate) fn command_error(n64: &mut N64, channel: usize) {
    if let Some(rx) = n64
        .pif
        .channels
        .get(channel)
        .and_then(|channel| channel.buffers.rx)
    {
        if let Some(length) = n64.pif.memory.ram.get_mut(rx) {
            *length |= CHANNEL_NO_DEVICE;
        }
    }
}

pub(crate) fn validate_command(
    n64: &mut N64,
    channel: usize,
    minimum_tx: usize,
    minimum_rx: usize,
) -> bool {
    let Some(buffers) = n64.pif.channels.get(channel).map(|channel| channel.buffers) else {
        return false;
    };
    let valid = match (buffers.tx, buffers.rx, buffers.tx_buf, buffers.rx_buf) {
        (Some(tx), Some(rx), Some(tx_buf), Some(rx_buf))
            if rx < PIF_RAM_COMMAND_ADDR && tx < PIF_RAM_COMMAND_ADDR =>
        {
            let tx_length = (n64.pif.memory.ram[tx] & CHANNEL_LENGTH_MASK) as usize;
            let rx_length = (n64.pif.memory.ram[rx] & CHANNEL_LENGTH_MASK) as usize;
            tx_length >= minimum_tx
                && rx_length >= minimum_rx
                && tx_buf + tx_length == rx_buf
                && rx_buf + rx_length <= PIF_RAM_COMMAND_ADDR
        }
        _ => false,
    };
    if !valid {
        command_error(n64, channel);
    }
    valid
}

fn setup_pif_channel(n64: &mut N64, channel: usize, buf: usize) -> Option<usize> {
    let tx = n64.pif.memory.ram[buf] & CHANNEL_LENGTH_MASK;
    let rx = n64.pif.memory.ram[buf + 1] & CHANNEL_LENGTH_MASK;
    let length = 2 + tx as usize + rx as usize;
    if tx == 0 || buf + length > PIF_RAM_COMMAND_ADDR {
        n64.pif.memory.ram[buf + 1] |= CHANNEL_NO_DEVICE;
        return None;
    }
    n64.pif.channels[channel].configure(buf, tx);
    Some(length)
}

fn setup_channels_format(n64: &mut N64) {
    n64.pif.channels.iter_mut().for_each(PifChannel::disable);
    let mut ram_idx = 0;
    let mut channel_idx = 0;

    while ram_idx < PIF_RAM_SIZE && channel_idx < PIF_CHANNELS_COUNT {
        match n64.pif.memory.ram[ram_idx] {
            RAM_EMPTY | RAM_DISABLE => {
                n64.pif.channels[channel_idx].disable();
                channel_idx += 1;
                ram_idx += 1;
            }
            RAM_SKIP => ram_idx += 1,
            RAM_TERMINATOR => {
                n64.pif.channels[channel_idx..PIF_CHANNELS_COUNT]
                    .iter_mut()
                    .for_each(PifChannel::disable);
                break;
            }
            _ => {
                if ram_idx + 1 < PIF_RAM_SIZE && n64.pif.memory.ram[ram_idx + 1] == RAM_TERMINATOR {
                    ram_idx += 1;
                    continue;
                }

                if ram_idx + 2 >= PIF_RAM_SIZE {
                    break;
                }

                let Some(length) = setup_pif_channel(n64, channel_idx, ram_idx) else {
                    break;
                };
                ram_idx += length;
                channel_idx += 1;
            }
        }
    }
}

pub fn process_ram(n64: &mut N64) {
    let mut clrmask = 0;
    let command = n64.pif.memory.ram[PIF_RAM_COMMAND_ADDR];

    for cmd in PifCommand::ALL {
        let mask = cmd as u8;

        if command & mask != 0 {
            match cmd {
                PifCommand::SetupChannels => {
                    setup_channels_format(n64);
                    clrmask |= mask;
                }
                PifCommand::ProcessChallenge => {
                    n64.pif.channels[..PIF_CHANNELS_COUNT]
                        .iter_mut()
                        .for_each(PifChannel::disable);
                    process_cic_challenge(n64);
                    clrmask |= mask;
                }
                PifCommand::Unknown => {
                    clrmask |= mask;
                }
                PifCommand::ClearRom => {
                    n64.pif.memory.rom.fill(0);
                }
                PifCommand::SetStatusBit => {
                    n64.pif.memory.ram[PIF_RAM_COMMAND_ADDR] = STATUS_READY;
                }
            }
        }
    }

    n64.pif.memory.ram[PIF_RAM_COMMAND_ADDR] &= !clrmask;
}

pub fn set_cartridge_region(n64: &mut N64) {
    n64.pif.memory.rom = match n64.cartridge.boot.pal {
        true => PAL_PIF_ROM,
        false => NTSC_PIF_ROM,
    };

    [PIF_CIC_SEED_ADDR1, PIF_CIC_SEED_ADDR2]
        .iter()
        .for_each(|&addr| n64.pif.memory.ram[addr] = n64.cartridge.boot.cic_seed);
}

fn process_cic_challenge(n64: &mut N64) {
    let mut challenge = [0u8; CIC_NIBBLE_COUNT];
    let mut response = [0u8; CIC_NIBBLE_COUNT];

    challenge
        .chunks_mut(2)
        .zip(n64.pif.memory.ram[CIC_RAM_OFFSET..CIC_RAM_OFFSET + CIC_CHALLENGE_SIZE].iter())
        .for_each(|(chunk, &byte)| {
            chunk[0] = (byte >> 4) & 0xf;
            chunk[1] = byte & 0xf;
        });

    n64_cic_nus_6105(challenge, &mut response, CIC_RESPONSE_LENGTH);

    n64.pif.memory.ram[CIC_STATUS_OFFSET..CIC_RAM_OFFSET].fill(0);

    n64.pif.memory.ram[CIC_RAM_OFFSET..CIC_RAM_OFFSET + CIC_CHALLENGE_SIZE]
        .iter_mut()
        .zip(response.chunks(2))
        .for_each(|(byte, nibbles)| *byte = (nibbles[0] << 4) | nibbles[1]);
}

fn n64_cic_nus_6105(
    challenge: [u8; CIC_NIBBLE_COUNT],
    response: &mut [u8; CIC_NIBBLE_COUNT],
    len: usize,
) {
    let mut key = 0xB;
    let mut current_lut = &LUT0;

    for i in 0..len {
        response[i] = (key + 5 * challenge[i]) & 0xF;
        key = current_lut[response[i] as usize];

        let sign = (response[i] >> 3) & 0x1;
        let magnitude = if sign == 1 { !response[i] } else { response[i] } & 0x7;
        let mod_value = if magnitude % 3 == 1 { sign } else { 1 - sign };

        current_lut = match (current_lut, response[i]) {
            (&LUT1, 0x1 | 0x9) => &LUT1,
            (&LUT1, 0xB | 0xE) => &LUT0,
            _ if mod_value == 1 => &LUT1,
            _ => &LUT0,
        };
    }
}
