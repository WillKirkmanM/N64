use crate::storage::SaveTypes;

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

pub fn get_save_type(game_id: &str) -> Vec<SaveTypes> {
    match game_id {
        id if EEPROM4K_GAMES.contains(&id) => vec![SaveTypes::Eeprom4k],
        id if EEPROM16K_GAMES.contains(&id) => vec![SaveTypes::Eeprom16k],
        id if FLASH_GAMES.contains(&id) => vec![SaveTypes::Flash],
        "NPQ" => vec![],
        _ => vec![SaveTypes::Eeprom4k, SaveTypes::Sram],
    }
}
