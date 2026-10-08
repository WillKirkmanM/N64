use super::{JCmd, Jdt, EEPROM_BLOCK_SIZE, EEPROM_MAX_SIZE};
use crate::N64;
use p64_gui::storage::SaveTypes;

pub fn process(n64: &mut N64, channel: usize) {
    if !crate::pif::validate_command(n64, channel, 1, 0) {
        return;
    }
    let cmd = n64.pif.memory.ram[n64.pif.channels[channel].tx_buffer()];

    let lengths = match cmd {
        x if x == JCmd::Reset as u8 || x == JCmd::Status as u8 || x == JCmd::AfRtcStatus as u8 => {
            Some((1, 3))
        }
        x if x == JCmd::EepromRead as u8 => Some((2, EEPROM_BLOCK_SIZE)),
        x if x == JCmd::EepromWrite as u8 => Some((2 + EEPROM_BLOCK_SIZE, 1)),
        x if x == JCmd::AfRtcRead as u8 => Some((2, 9)),
        x if x == JCmd::AfRtcWrite as u8 => Some((10, 1)),
        _ => None,
    };
    let Some((tx, rx)) = lengths else {
        crate::pif::command_error(n64, channel);
        return;
    };
    if !crate::pif::validate_command(n64, channel, tx, rx) {
        return;
    }
    match cmd {
        x if x == JCmd::Reset as u8 || x == JCmd::Status as u8 => {
            handle_status_or_reset(n64, channel)
        }
        x if x == JCmd::EepromRead as u8 => handle_eeprom_read(n64, channel),
        x if x == JCmd::EepromWrite as u8 => handle_eeprom_write(n64, channel),
        x if x == JCmd::AfRtcStatus as u8 => handle_af_rtc_status(n64, channel),
        x if x == JCmd::AfRtcRead as u8 => handle_af_rtc_read(n64, channel),
        x if x == JCmd::AfRtcWrite as u8 => handle_af_rtc_write(n64, channel),
        _ => {}
    }
}

fn handle_status_or_reset(n64: &mut N64, channel: usize) {
    let eeprom_type = if n64.has_save_type(SaveTypes::Eeprom16k) {
        Jdt::Eeprom16k
    } else if n64.has_save_type(SaveTypes::Eeprom4k) {
        Jdt::Eeprom4k
    } else {
        n64.pif.memory.ram[n64.pif.channels[channel].buffers.rx.unwrap()] |= 0x80;
        return;
    };

    let rx_buf = n64.pif.channels[channel].rx_buffer();
    n64.pif.memory.ram[rx_buf] = eeprom_type as u8;
    n64.pif.memory.ram[rx_buf + 1] = (eeprom_type as u16 >> 8) as u8;
    n64.pif.memory.ram[rx_buf + 2] = 0;
}

fn handle_eeprom_read(n64: &mut N64, channel: usize) {
    eeprom_read_block(
        n64,
        n64.pif.channels[channel].tx_buffer() + 1,
        n64.pif.channels[channel].rx_buffer(),
    )
}

fn handle_eeprom_write(n64: &mut N64, channel: usize) {
    eeprom_write_block(
        n64,
        n64.pif.channels[channel].tx_buffer() + 1,
        n64.pif.channels[channel].tx_buffer() + 2,
        n64.pif.channels[channel].rx_buffer(),
    )
}

fn handle_af_rtc_status(n64: &mut N64, channel: usize) {
    let rx_buf = n64.pif.channels[channel].rx_buffer();
    n64.pif.memory.ram[rx_buf..rx_buf + 2].copy_from_slice(&(Jdt::AfRtc as u16).to_le_bytes());
    n64.pif.memory.ram[rx_buf + 2] = n64.cartridge.rtc.status();
}

fn handle_af_rtc_read(n64: &mut N64, channel: usize) {
    af_rtc_read_block(
        n64,
        n64.pif.channels[channel].tx_buffer() + 1,
        n64.pif.channels[channel].rx_buffer(),
        n64.pif.channels[channel].rx_buffer() + 8,
    );
}

fn handle_af_rtc_write(n64: &mut N64, channel: usize) {
    af_rtc_write_block(
        n64,
        n64.pif.channels[channel].tx_buffer() + 1,
        n64.pif.channels[channel].tx_buffer() + 2,
        n64.pif.channels[channel].rx_buffer(),
    );
}

fn format_eeprom(bytes: &mut Vec<u8>) {
    if bytes.len() < EEPROM_MAX_SIZE {
        bytes.resize(EEPROM_MAX_SIZE, 0xff);
    }
}

fn eeprom_read_block(n64: &mut N64, block: usize, offset: usize) {
    let address = n64.pif.memory.ram[block] as usize * EEPROM_BLOCK_SIZE;
    let buffers = n64.eeprom_buffers();
    format_eeprom(buffers.save);
    buffers.packet[offset..offset + EEPROM_BLOCK_SIZE]
        .copy_from_slice(&buffers.save[address..address + EEPROM_BLOCK_SIZE]);
}

fn eeprom_write_block(n64: &mut N64, block: usize, offset: usize, status: usize) {
    let address = n64.pif.memory.ram[block] as usize * EEPROM_BLOCK_SIZE;
    let buffers = n64.eeprom_buffers();
    format_eeprom(buffers.save);
    buffers.save[address..address + EEPROM_BLOCK_SIZE]
        .copy_from_slice(&buffers.packet[offset..offset + EEPROM_BLOCK_SIZE]);
    buffers.packet[status] = 0;
    *buffers.dirty = true;
}

fn af_rtc_read_block(n64: &mut N64, block: usize, offset: usize, status: usize) {
    let Some(data) = n64.cartridge.rtc.read(n64.pif.memory.ram[block]) else {
        crate::pif::command_error(n64, crate::pif::CART_CHANNEL);
        return;
    };
    n64.pif.memory.ram[offset..offset + EEPROM_BLOCK_SIZE].copy_from_slice(&data);
    n64.pif.memory.ram[status] = n64.cartridge.rtc.status();
}

fn af_rtc_write_block(n64: &mut N64, block: usize, offset: usize, status: usize) {
    let data = n64.pif.memory.ram[offset..offset + EEPROM_BLOCK_SIZE]
        .try_into()
        .unwrap();
    if n64.cartridge.rtc.write(n64.pif.memory.ram[block], data) {
        n64.pif.memory.ram[status] = n64.cartridge.rtc.status();
    } else {
        crate::pif::command_error(n64, crate::pif::CART_CHANNEL);
    }
}
