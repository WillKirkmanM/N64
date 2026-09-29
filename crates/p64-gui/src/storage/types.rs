use serde_json::Map;
use std::path::PathBuf;

#[derive(Copy, Clone, PartialEq)]
pub enum SaveTypes {
    Eeprom4k,
    Eeprom16k,
    Flash,
    Mempak,
    Romsave,
    Sram,
}

#[derive(Default)]
pub struct SaveFile<T> {
    pub data: T,
    pub dirty: bool,
    pub path: PathBuf,
}

#[derive(Default)]
pub struct SaveData {
    pub eeprom: SaveFile<Vec<u8>>,
    pub sram: SaveFile<Vec<u8>>,
    pub flash: SaveFile<Vec<u8>>,
    pub mempak: SaveFile<Vec<u8>>,
    pub romsave: SaveFile<Map<String, serde_json::Value>>,
}
