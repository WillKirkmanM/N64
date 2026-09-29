use super::{SaveData, SaveTypes};
use serde_json::json;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn rom_save_persistence_retains_legacy_tuple_json() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("p64-romsave-{}-{nonce}", std::process::id()));
    fs::create_dir(&directory).unwrap();

    let mut saved = SaveData::default();
    saved.set_paths(&directory, "NTEST", "rom-hash");
    saved.romsave.data = serde_json::from_value(json!({"offset": 17, "bytes": [1, 3, 5]})).unwrap();
    saved.romsave.dirty = true;
    saved.write(SaveTypes::Romsave).unwrap();

    let path = directory.join("NTEST-rom-hash.romsave");
    let document: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();

    assert_eq!(document, json!([{"offset": 17, "bytes": [1, 3, 5]}, true]));

    let mut loaded = SaveData::default();
    loaded.set_paths(&directory, "NTEST", "rom-hash");
    loaded.load().unwrap();

    assert_eq!(loaded.romsave.data, saved.romsave.data);
    assert!(!loaded.romsave.dirty);

    fs::remove_file(path).unwrap();
    fs::remove_dir(directory).unwrap();
}

#[test]
fn failed_save_remains_dirty_and_does_not_block_other_saves() {
    let directory = std::env::temp_dir().join(format!("p64-save-retry-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let mut saved = SaveData::default();
    saved.set_paths(&directory, "TEST", "hash");
    fs::create_dir(&saved.eeprom.path).unwrap();
    saved.eeprom.data = vec![1, 2];
    saved.eeprom.dirty = true;
    saved.sram.data = vec![3, 4];
    saved.sram.dirty = true;
    assert!(saved.write_dirty().is_err());
    assert!(saved.eeprom.dirty);
    assert!(!saved.sram.dirty);
    assert_eq!(fs::read(&saved.sram.path).unwrap(), vec![3, 4]);
    fs::remove_dir(&saved.eeprom.path).unwrap();
    saved.write_dirty().unwrap();
    assert!(!saved.eeprom.dirty);
    assert_eq!(fs::read(&saved.eeprom.path).unwrap(), vec![1, 2]);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn preparing_distinct_roms_loads_only_identity_named_saves() {
    let directory = std::env::temp_dir().join(format!("p64-rom-identities-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let anonymous = directory.join("-.sra");
    fs::write(&anonymous, [99, 99]).unwrap();
    fs::write(directory.join("NZL-firsthash.sra"), [11, 12]).unwrap();
    fs::write(directory.join("NKT-secondhash.sra"), [21, 22]).unwrap();
    let mut first = SaveData::default();
    first.prepare_game(&directory, "NZL", "firsthash").unwrap();
    let mut second = SaveData::default();
    second
        .prepare_game(&directory, "NKT", "secondhash")
        .unwrap();
    assert_eq!(first.sram.path, directory.join("NZL-firsthash.sra"));
    assert_eq!(second.sram.path, directory.join("NKT-secondhash.sra"));
    assert_ne!(first.sram.path, second.sram.path);
    assert_eq!(first.sram.data, [11, 12]);
    assert_eq!(second.sram.data, [21, 22]);
    first.sram.data = vec![31, 32];
    first.sram.dirty = true;
    first.write_dirty().unwrap();
    assert_eq!(fs::read(&second.sram.path).unwrap(), [21, 22]);
    assert_eq!(fs::read(&anonymous).unwrap(), [99, 99]);
    fs::remove_dir_all(directory).unwrap();
}
