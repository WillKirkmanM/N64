use super::*;

#[test]
fn persisted_rom_patches_keep_valid_bytes_and_ignore_malformed_entries() {
    let mut cartridge = Cartridge {
        rom: Memory::from(vec![0x11; 4]),
        ..Cartridge::default()
    };
    let patches = serde_json::json!({
        "0": 0,
        "1": 255,
        "2": 256,
        "3": -1,
        "4": 42,
        "invalid": 77,
        "999999999999999999999999": 88
    });
    cartridge.restore_rom(patches.as_object().unwrap());

    assert_eq!(&*cartridge.rom.data, &[0, 255, 0x11, 0x11]);
}
