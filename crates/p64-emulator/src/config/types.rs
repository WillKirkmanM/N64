use p64_gui::input::CONTROLLER_COUNT;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const DEFAULT_RESOLUTION: (u32, u32) = (1280, 720);
pub const UNASSIGNED_CONTROLLER: i32 = -1;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub rom_folder: Option<PathBuf>,
    pub settings: Option<Settings>,
    pub controller_settings: Option<ControllerSettings>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    pub graphics: GraphicsSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsSettings {
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub vsync: bool,
    pub upscaling: u8,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            width: DEFAULT_RESOLUTION.0,
            height: DEFAULT_RESOLUTION.1,
            fullscreen: false,
            vsync: true,
            upscaling: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerSettings {
    #[serde(default)]
    pub controller_guids: Option<[Option<String>; CONTROLLER_COUNT]>,
    pub selected_controller: [i32; CONTROLLER_COUNT],
    pub selected_profile: [String; CONTROLLER_COUNT],
}

impl Default for ControllerSettings {
    fn default() -> Self {
        Self {
            controller_guids: Some(Default::default()),
            selected_controller: [UNASSIGNED_CONTROLLER; CONTROLLER_COUNT],
            selected_profile: std::array::from_fn(|_| "Default".to_owned()),
        }
    }
}
