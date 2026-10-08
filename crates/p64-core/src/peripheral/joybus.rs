use super::{JoyCommand, CONTROLLER_FLAGS, PAK_CHUNK_SIZE};
use crate::N64;
use p64_gui::input;

const PAK_LOW_ADDRESS_MASK: u8 = 0xe0;
const CRC_HIGH_BIT: u8 = 0x80;
const CRC_POLYNOMIAL: u8 = 0x85;

pub fn process(n64: &mut N64, channel: usize) {
    if !crate::pif::validate_command(n64, channel, 1, 0) {
        return;
    }
    let cmd = n64.pif.memory.ram[n64.pif.channels[channel].tx_buffer()];
    let Some(cmd) = JoyCommand::from_byte(cmd) else {
        crate::pif::command_error(n64, channel);
        return;
    };

    let (tx, rx) = match cmd {
        JoyCommand::Status | JoyCommand::Reset => (1, 3),
        JoyCommand::ControllerRead => (1, 4),
        JoyCommand::PakRead => (3, PAK_CHUNK_SIZE + 1),
        JoyCommand::PakWrite => (3 + PAK_CHUNK_SIZE, 1),
    };
    if !crate::pif::validate_command(n64, channel, tx, rx) {
        return;
    }
    match cmd {
        JoyCommand::Status | JoyCommand::Reset => handle_status(n64, channel),
        JoyCommand::ControllerRead => handle_read(n64, channel),
        JoyCommand::PakRead => handle_pak_read(n64, channel),
        JoyCommand::PakWrite => handle_pak_write(n64, channel),
    }
}

pub(super) fn handle_status(n64: &mut N64, channel: usize) {
    let rx_buf = n64.pif.channels[channel].rx_buffer();
    n64.pif.memory.ram[rx_buf..rx_buf + 2].copy_from_slice(&CONTROLLER_FLAGS.to_le_bytes());
    n64.pif.memory.ram[rx_buf + 2] = if n64.pif.channels[channel].pak_handler.is_some() {
        1
    } else {
        0
    };
}

fn handle_read(n64: &mut N64, channel: usize) {
    let offset = n64.pif.channels[channel].rx_buffer();
    let input = input::get(&mut n64.ui, channel);
    n64.pif.memory.ram[offset..offset + 4].copy_from_slice(&controller_input_bytes(input));
}

pub(super) fn controller_input_bytes(input: u32) -> [u8; 4] {
    // Input::AButton is bit 7, C buttons are bits 8..11, and the signed
    // stick coordinates occupy bits 16..23 and 24..31 of the UI word.
    input.to_le_bytes()
}

fn handle_pak_read(n64: &mut N64, channel: usize) {
    let (addr_offset, data_offset) = get_pak_offsets(n64, channel);
    let address = get_pak_address(n64, addr_offset);

    if let Some(handler) = n64.pif.channels[channel].pak_handler {
        (handler.read)(n64, channel, address, data_offset, PAK_CHUNK_SIZE);
        n64.pif.memory.ram[data_offset + PAK_CHUNK_SIZE] = calculate_crc(n64, data_offset);
    }
}

fn handle_pak_write(n64: &mut N64, channel: usize) {
    let tx = n64.pif.channels[channel].tx_buffer();
    let data_offset = tx + 3;
    let address = get_pak_address(n64, tx + 1);
    if let Some(handler) = n64.pif.channels[channel].pak_handler {
        (handler.write)(n64, channel, address, data_offset, PAK_CHUNK_SIZE);
        let crc_offset = n64.pif.channels[channel].rx_buffer();
        n64.pif.memory.ram[crc_offset] = calculate_crc(n64, data_offset);
    }
}

#[inline]
fn get_pak_address(n64: &N64, addr_offset: usize) -> u16 {
    u16::from_be_bytes([
        n64.pif.memory.ram[addr_offset],
        n64.pif.memory.ram[addr_offset + 1] & PAK_LOW_ADDRESS_MASK,
    ])
}

#[inline]
fn get_pak_offsets(n64: &N64, channel: usize) -> (usize, usize) {
    let tx = n64.pif.channels[channel].tx_buffer();
    let rx = n64.pif.channels[channel].rx_buffer();

    (tx + 1, rx)
}

fn calculate_crc(n64: &N64, data_offset: usize) -> u8 {
    n64.pif.memory.ram[data_offset..data_offset + PAK_CHUNK_SIZE]
        .iter()
        .copied()
        .chain(std::iter::once(0))
        .fold(0u8, |crc, byte| {
            (0..8).fold(crc, |crc, bit| {
                let tap = if crc & CRC_HIGH_BIT != 0 {
                    CRC_POLYNOMIAL
                } else {
                    0
                };
                ((crc << 1) | u8::from(byte & (CRC_HIGH_BIT >> bit) != 0)) ^ tap
            })
        })
}
