use p64_gui::input::CONTROLLER_COUNT;
use p64_gui::ui::Ui;

mod types;
pub use types::{Config, ControllerSettings, GraphicsSettings, Settings, UNASSIGNED_CONTROLLER};

pub fn save_config(
    game_ui: &mut Ui,
    selected_controller: [i32; CONTROLLER_COUNT],
    selected_profile: [String; CONTROLLER_COUNT],
) {
    let joysticks = game_ui.joystick_subsystem();
    let assignments = selected_controller.map(|index| {
        u32::try_from(index)
            .ok()
            .and_then(|index| joysticks.device_guid(index).ok())
            .map(|guid| guid.to_string())
    });
    let input = game_ui.input_config_mut();
    input.controller_assignment = assignments;
    input.input_profile_binding = selected_profile;
    game_ui.rebind_input();
}

impl Config {
    pub fn load(directory: &std::path::Path) -> std::io::Result<Self> {
        let current = directory.join("launcher.json");
        match std::fs::read(current) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                match std::fs::read(directory.join("config.json")) {
                    Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        Ok(Self::default())
                    }
                    Err(error) => Err(error),
                }
            }
            Err(error) => Err(error),
        }
    }

    pub fn save(&self, directory: &std::path::Path) -> std::io::Result<()> {
        p64_gui::storage::atomic_write(
            &directory.join("launcher.json"),
            &serde_json::to_vec_pretty(self)?,
        )
    }
}

pub fn apply_bindings(
    game_ui: &mut Ui,
    assignments: [Option<String>; CONTROLLER_COUNT],
    profiles: [String; CONTROLLER_COUNT],
) {
    let input = game_ui.input_config_mut();
    input.controller_assignment = assignments;
    input.input_profile_binding = profiles.map(|profile| {
        if input.input_profiles.contains_key(&profile) {
            profile
        } else {
            "Default".to_owned()
        }
    });
    game_ui.rebind_input();
}
