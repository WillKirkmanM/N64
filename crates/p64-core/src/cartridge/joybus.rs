use super::*;

pub fn process(n64: &mut N64, channel: usize) {
    let cmd = n64.pif.ram[n64.pif.channels[channel].tx_buf.unwrap()];

    match cmd {
        x if x == JCmd::Reset as u8 || x == JCmd::Status as u8 => {
            handle_status_or_reset(n64, channel)
        }
        x if x == JCmd::EepromRead as u8 => handle_eeprom_read(n64, channel),
        x if x == JCmd::EepromWrite as u8 => handle_eeprom_write(n64, channel),
        x if x == JCmd::AfRtcStatus as u8 => handle_af_rtc_status(n64, channel),
        x if x == JCmd::AfRtcRead as u8 => handle_af_rtc_read(n64, channel),
        x if x == JCmd::AfRtcWrite as u8 => handle_af_rtc_write(n64, channel),
        _ => panic!("unknown cart command"),
    }
}

pub(super) fn handle_status_or_reset(n64: &mut N64, channel: usize) {
    let eeprom_type = if n64.ui.save_data.save_type.contains(&SaveTypes::Eeprom16k) {
        Jdt::Eeprom16k
    } else if n64.ui.save_data.save_type.contains(&SaveTypes::Eeprom4k) {
        Jdt::Eeprom4k
    } else {
        n64.pif.ram[n64.pif.channels[channel].rx.unwrap()] |= 0x80;
        return;
    };

    let rx_buf = n64.pif.channels[channel].rx_buf.unwrap();
    n64.pif.ram[rx_buf] = eeprom_type as u8;
    n64.pif.ram[rx_buf + 1] = (eeprom_type as u16 >> 8) as u8;
    n64.pif.ram[rx_buf + 2] = 0;
}

pub(super) fn handle_eeprom_read(n64: &mut N64, channel: usize) {
    eeprom_read_block(
        n64,
        n64.pif.channels[channel].tx_buf.unwrap() + 1,
        n64.pif.channels[channel].rx_buf.unwrap(),
    )
}

pub(super) fn handle_eeprom_write(n64: &mut N64, channel: usize) {
    eeprom_write_block(
        n64,
        n64.pif.channels[channel].tx_buf.unwrap() + 1,
        n64.pif.channels[channel].tx_buf.unwrap() + 2,
        n64.pif.channels[channel].rx_buf.unwrap(),
    )
}

pub(super) fn handle_af_rtc_status(n64: &mut N64, channel: usize) {
    let rx_buf = n64.pif.channels[channel].rx_buf.unwrap();
    n64.pif.ram[rx_buf] = Jdt::AfRtc as u8;
    n64.pif.ram[rx_buf + 1] = (Jdt::AfRtc as u16 >> 8) as u8;
    n64.pif.ram[rx_buf + 2] = 0x00;
}

pub(super) fn handle_af_rtc_read(n64: &mut N64, channel: usize) {
    af_rtc_read_block(
        n64,
        n64.pif.channels[channel].tx_buf.unwrap() + 1,
        n64.pif.channels[channel].rx_buf.unwrap(),
        n64.pif.channels[channel].rx_buf.unwrap() + 8,
    );
}

pub(super) fn handle_af_rtc_write(n64: &mut N64, channel: usize) {
    af_rtc_write_block(
        n64,
        n64.pif.channels[channel].tx_buf.unwrap() + 1,
        n64.pif.channels[channel].tx_buf.unwrap() + 2,
        n64.pif.channels[channel].rx_buf.unwrap(),
    );
}

pub(super) fn format_eeprom(n64: &mut N64) {
    if n64.ui.save_data.data.eeprom.data.len() < EEPROM_MAX_SIZE {
        n64.ui
            .save_data
            .data
            .eeprom
            .data
            .resize(EEPROM_MAX_SIZE, 0xFF)
    }
}

pub(super) fn eeprom_read_block(n64: &mut N64, block: usize, offset: usize) {
    let address = n64.pif.ram[block] as usize * EEPROM_BLOCK_SIZE;

    format_eeprom(n64);

    n64.pif.ram[offset..offset + EEPROM_BLOCK_SIZE]
        .copy_from_slice(&n64.ui.save_data.data.eeprom.data[address..address + EEPROM_BLOCK_SIZE]);
}

pub(super) fn eeprom_write_block(n64: &mut N64, block: usize, offset: usize, status: usize) {
    let address = n64.pif.ram[block] as usize * EEPROM_BLOCK_SIZE;

    format_eeprom(n64);

    n64.ui.save_data.data.eeprom.data[address..address + EEPROM_BLOCK_SIZE]
        .copy_from_slice(&n64.pif.ram[offset..offset + EEPROM_BLOCK_SIZE]);

    n64.pif.ram[status] = 0x00;

    n64.ui.save_data.data.eeprom.dirty = true
}

pub(super) fn af_rtc_read_block(n64: &mut N64, block: usize, offset: usize, status: usize) {
    match n64.pif.ram[block] {
        0 => {
            n64.pif.ram[offset] = n64.cartridge.rtc.control as u8;
            n64.pif.ram[offset + 1] = (n64.cartridge.rtc.control >> 8) as u8;
            n64.pif.ram[status] = 0x00;
        }
        1 => panic!("AF-RTC reading block 1 is not implemented!"),
        2 => {
            time2data(n64, offset);
            n64.pif.ram[status] = 0x00;
        }
        _ => panic!("AF-RTC read invalid block"),
    }
}

pub(super) fn af_rtc_write_block(n64: &mut N64, block: usize, offset: usize, status: usize) {
    match n64.pif.ram[block] {
        0 => {
            n64.cartridge.rtc.control =
                (n64.pif.ram[offset + 1] as u16) << 8 | n64.pif.ram[offset] as u16;
            n64.pif.ram[status] = 0x00;
        }
        1 => {
            if (n64.cartridge.rtc.control & 0x01) != 0 {
                return;
            }
            panic!("AF-RTC writing block 1 is not implemented!");
        }
        2 => {
            if (n64.cartridge.rtc.control & 0x02) != 0 {
                return;
            }
            panic!("AF-RTC writing block 2 is not implemented!");
        }
        _ => panic!("AF-RTC write invalid block"),
    }
}
