use crate::{config, input, storage};

pub struct Ui {
    pub config: Config,
    pub game_info: GameInfo,
    pub input_controllers: InputControllers,
    pub save_data: SaveData,
    pub sdl_context: SdlContext,
    pub window_audio: WindowAudio,
}

pub struct Config {
    pub(crate) original: Vec<u8>,
    pub file_path: std::path::PathBuf,
    pub config: config::Config,
}

pub struct GameInfo {
    pub id: String,
    pub hash: String,
}

pub struct InputControllers {
    pub controllers: [input::Controllers; input::CONTROLLER_COUNT],
    pub joystick_subsystem: Option<sdl2::JoystickSubsystem>,
    pub controller_subsystem: Option<sdl2::GameControllerSubsystem>,
}

pub struct SaveData {
    pub save_type: Vec<storage::SaveTypes>,
    pub data: storage::SaveData,
}

pub struct SdlContext {
    pub sdl_context: Option<sdl2::Sdl>,
    pub video_subsystem: Option<sdl2::VideoSubsystem>,
    pub audio_subsystem: Option<sdl2::AudioSubsystem>,
}

pub struct WindowAudio {
    pub window: Option<sdl2::video::Window>,
    pub audio_device: Option<sdl2::audio::AudioQueue<i16>>,
}

impl SdlContext {
    pub fn event_pump(&self) -> Result<sdl2::EventPump, String> {
        self.sdl_context
            .as_ref()
            .ok_or_else(|| "SDL context unavailable".to_owned())?
            .event_pump()
    }

    pub fn joystick(&self) -> Result<sdl2::JoystickSubsystem, String> {
        self.sdl_context
            .as_ref()
            .ok_or_else(|| "SDL context unavailable".to_owned())?
            .joystick()
    }
}
