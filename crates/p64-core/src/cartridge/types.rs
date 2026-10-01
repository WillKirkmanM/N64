use crate::builder::memory::Memory;

#[path = "boot.rs"]
mod boot;
pub use boot::CartridgeBoot;

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
    pub latch: u32,
    pub boot: CartridgeBoot,
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

#[path = "rtc.rs"]
mod rtc;
pub use rtc::AfRtc;

impl Cartridge {
    pub(crate) fn restore_rom(&mut self, patches: &serde_json::Map<String, serde_json::Value>) {
        for (address, value) in patches {
            let Ok(address) = address.parse::<usize>() else {
                continue;
            };

            let Some(value) = value.as_u64().and_then(|value| u8::try_from(value).ok()) else {
                continue;
            };

            if let Some(destination) = self.rom.data.get_mut(address) {
                *destination = value;
            }
        }
    }
}
