use super::*;

#[derive(Clone, Debug, PartialEq, Default)]
pub enum CicType {
    #[default]
    CicNus6101,
    CicNus6102,
    CicNus6103,
    CicNus6105,
    CicNus6106,
    CicNus5167,
}

#[derive(Clone, Default)]
pub struct Cartridge {
    pub rom: Memory,
    pub pal: bool,
    pub latch: u32,
    pub cic_type: CicType,
    pub cic_seed: u8,
    pub rdram_size_offset: usize,
    pub rtc: AfRtc,
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum JCmd {
    Status = 0x00,
    EepromRead = 0x04,
    EepromWrite = 0x05,
    AfRtcStatus = 0x06,
    AfRtcRead = 0x07,
    AfRtcWrite = 0x08,
    Reset = 0xff,
}

#[repr(u16)]
#[derive(Copy, Clone)]
pub enum Jdt {
    AfRtc = 0x1000,
    Eeprom4k = 0x8000,
    Eeprom16k = 0xc000,
}
