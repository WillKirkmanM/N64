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
