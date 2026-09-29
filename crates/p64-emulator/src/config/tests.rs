use super::*;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn legacy_combined_config_migrates_without_losing_either_schema() {
    let directory = std::env::temp_dir().join(format!(
        "p64-config-migration-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&directory).unwrap();
    let native = p64_gui::config::Config::default();
    let mut document = serde_json::to_value(&native).unwrap();
    document["rom_folder"] = serde_json::json!("roms");
    document["settings"] = serde_json::to_value(Settings::default()).unwrap();
    let legacy = serde_json::to_vec(&document).unwrap();
    fs::write(directory.join("config.json"), &legacy).unwrap();
    let mut launcher = Config::load(&directory).unwrap();
    let mut inputs = p64_gui::config::Config::load(&directory).unwrap();
    assert_eq!(launcher.rom_folder, Some("roms".into()));
    assert!(inputs.input.input_profiles.contains_key("Default"));
    inputs.input.input_profile_binding[0] = "Custom".to_owned();
    inputs.save(&directory.join("input.json")).unwrap();
    launcher.settings.as_mut().unwrap().graphics.vsync = false;
    launcher.save(&directory).unwrap();
    assert_eq!(fs::read(directory.join("config.json")).unwrap(), legacy);
    assert_eq!(
        p64_gui::config::Config::load(&directory)
            .unwrap()
            .input
            .input_profile_binding[0],
        "Custom"
    );
    assert!(
        !Config::load(&directory)
            .unwrap()
            .settings
            .unwrap()
            .graphics
            .vsync
    );
    fs::remove_dir_all(directory).unwrap();
}
