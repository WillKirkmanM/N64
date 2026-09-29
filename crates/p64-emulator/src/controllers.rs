use p64_gui::ui::Ui;

mod device;
pub use device::ConnectedController;

pub fn get_input_profiles(game_ui: &Ui) -> Vec<String> {
    game_ui
        .input_config()
        .input_profiles
        .keys()
        .cloned()
        .collect()
}

pub fn get_controllers(game_ui: &Ui) -> Vec<ConnectedController> {
    let joysticks = game_ui.joystick_subsystem();
    (0..joysticks.num_joysticks().unwrap_or(0))
        .filter_map(|index| {
            let guid = joysticks.device_guid(index).ok()?.to_string();
            let name = joysticks.name_for_index(index).ok()?;
            Some(ConnectedController { guid, name })
        })
        .collect()
}
