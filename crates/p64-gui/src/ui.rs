use std::fs::create_dir_all;
use std::fs::read;

use crate::input;
use crate::config;
use crate::storage;
use crate::storage::write_save;
use crate::storage::SaveTypes;
use dirs::data_dir;
use serde_json;
use serde_json::from_slice;

pub fn get_save_type(game_id: &str) -> Vec<SaveTypes> {
    const EEPROM4K_GAMES: [&str; 45] = [
        "NAB", "NAD", "NAL", "NBC", "NCG", "NCT", "NDU", "NFW", "NGE", "NGL", "NHS", "NHY", "NIJ",
        "NKG", "NKT", "NLR", "NM9", "NMF", "NML", "NMR", "NMW", "NMZ", "NOS", "NP2", "NPL", "NPM",
        "NPW", "NRA", "NRC", "NSI", "NSL", "NSM", "NSU", "NSV", "NTJ", "NTM", "NTN", "NTP", "NTR",
        "NTX", "NVL", "NVP", "NVY", "NWL", "NWR",
    ];

    const EEPROM16K_GAMES: [&str; 35] = [
        "NB7", "NGT", "NFU", "NCW", "NCZ", "ND6", "NDO", "ND2", "N3D", "NMX", "NGC", "NIM", "NNB",
        "NMV", "NM8", "NEV", "NPP", "NUB", "NPD", "NRZ", "NR7", "NEP", "NYS", "NAK", "NBK", "NDY",
        "NGU", "NJG", "NRP", "NWQ", "NYW", "NB5", "NBM", "NCR", "NDR",
    ];

    const FLASH_GAMES: [&str; 34] = [
        "NCC", "NDA", "NAF", "NJF", "NKJ", "NZS", "NM6", "NCK", "NMQ", "NPN", "NPF", "NPO", "CP2",
        "NP3", "NRH", "NSQ", "NT9", "NW4", "NDP", "NLB", "NMS", "NMT", "NRC", "NSM", "NSS", "NTC",
        "NTD", "NTE", "NTF", "NTG", "NTH", "NTI", "NTJ", "NTK",
    ];

    match game_id {
        id if EEPROM4K_GAMES.contains(&id) => vec![SaveTypes::Eeprom4k],
        id if EEPROM16K_GAMES.contains(&id) => vec![SaveTypes::Eeprom16k],
        id if FLASH_GAMES.contains(&id) => vec![SaveTypes::Flash],
        "NPQ" => vec![],
        _ => vec![SaveTypes::Eeprom4k, SaveTypes::Sram],
    }
}

pub fn load_saves(ui: &mut Ui) {
    let mut paths = [
        (
            &mut ui.save_data.data.eeprom_path,
            &mut ui.save_data.data.eeprom.0,
        ),
        (
            &mut ui.save_data.data.sram_path,
            &mut ui.save_data.data.sram.0,
        ),
        (
            &mut ui.save_data.data.flash_path,
            &mut ui.save_data.data.flash.0,
        ),
        (
            &mut ui.save_data.data.mempak_path,
            &mut ui.save_data.data.mempak.0,
        ),
    ];

    for (path, save) in paths.iter_mut() {
        if let Ok(data) = read(path) {
            **save = data;
        }
    }

    if let Ok(data) = read(&ui.save_data.data.romsave_path) {
        ui.save_data.data.romsave.0 = from_slice(&data).unwrap();
    }
}

pub fn write_saves(ui: &Ui) {
    [
        (SaveTypes::Eeprom16k, ui.save_data.data.eeprom.1),
        (SaveTypes::Sram, ui.save_data.data.sram.1),
        (SaveTypes::Flash, ui.save_data.data.flash.1),
        (SaveTypes::Mempak, ui.save_data.data.mempak.1),
        (SaveTypes::Romsave, ui.save_data.data.romsave.1),
    ]
    .iter()
    .filter(|&&(_, flag)| flag)
    .for_each(|&(save_type, _)| write_save(ui, save_type));
}

fn write_config(ui: &Ui) {
    let f = std::fs::File::create(ui.config.file_path.clone()).unwrap();
    serde_json::to_writer_pretty(f, &ui.config.config).unwrap();
}

mod types;
pub use types::{Ui, Config, GameInfo, InputControllers, SaveData, SdlContext, WindowAudio};
