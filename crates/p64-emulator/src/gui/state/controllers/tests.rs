use super::*;
use crate::config::{Config, Settings};
use std::fs;

#[test]
fn unrelated_settings_preserve_disconnected_guid_and_reconnection_resolves_display() {
    let directory =
        std::env::temp_dir().join(format!("p64-disconnected-binding-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let mut state = ControllerState::default();
    state.assignments[0] = Some("saved-controller".to_owned());
    state.resolve_connected();
    assert_eq!(state.selected_controller[0], UNASSIGNED_CONTROLLER);
    let mut config = Config {
        settings: Some(Settings::default()),
        controller_settings: Some(state.settings()),
        ..Config::default()
    };
    config.save(&directory).unwrap();
    config.settings.as_mut().unwrap().graphics.fullscreen = true;
    config.controller_settings = Some(state.settings());
    config.save(&directory).unwrap();
    let restored = Config::load(&directory).unwrap();
    state.assignments = restored
        .controller_settings
        .unwrap()
        .controller_guids
        .unwrap();
    assert_eq!(state.assignments[0].as_deref(), Some("saved-controller"));
    state.available = vec![
        ConnectedController {
            name: "Other".to_owned(),
            guid: "other-controller".to_owned(),
        },
        ConnectedController {
            name: "Reconnected".to_owned(),
            guid: "saved-controller".to_owned(),
        },
    ];
    state.resolve_connected();
    assert_eq!(state.selected_controller[0], 1);
    state.select_controller(0, UNASSIGNED_CONTROLLER);
    config.controller_settings = Some(state.settings());
    config.save(&directory).unwrap();
    assert!(Config::load(&directory)
        .unwrap()
        .controller_settings
        .unwrap()
        .controller_guids
        .unwrap()[0]
        .is_none());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn explicit_unassignment_clears_disconnected_binding_even_when_display_was_already_none() {
    let mut state = ControllerState::default();
    state.assignments[0] = Some("offline".to_owned());
    state.resolve_connected();
    state.select_controller(0, 99);
    assert_eq!(state.assignments[0].as_deref(), Some("offline"));
    state.select_controller(0, UNASSIGNED_CONTROLLER);
    assert!(state.assignments[0].is_none());
}
