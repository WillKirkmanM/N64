use super::*;

#[test]
fn default_headless_storage_is_in_memory_until_explicitly_prepared() {
    let mut ui = Ui::default();
    crate::ui::load_saves(&mut ui).unwrap();
    assert!(ui.save_data.data.sram.path.as_os_str().is_empty());
    assert!(ui.save_data.save_type.is_empty());
    let directory =
        std::env::temp_dir().join(format!("p64-headless-identity-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(directory.join("NZL-knownhash.sra"), [41, 42]).unwrap();
    ui.game_info.id = "NZL".to_owned();
    ui.game_info.hash = "knownhash".to_owned();
    crate::ui::load_saves(&mut ui).unwrap();
    assert!(ui.save_data.data.sram.path.as_os_str().is_empty());
    ui.prepare_saves(&directory).unwrap();
    assert_eq!(ui.save_data.data.sram.data, [41, 42]);
    assert!(ui
        .save_data
        .save_type
        .contains(&crate::storage::SaveTypes::Sram));
    assert_eq!(
        ui.save_data.data.sram.path,
        directory.join("NZL-knownhash.sra")
    );
    std::fs::remove_dir_all(directory).unwrap();
}
