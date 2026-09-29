use crate::config;
use crate::input;
use crate::storage;
use dirs::data_dir;

mod controllers;
mod types;
pub use crate::storage::get_save_type;
pub use types::{Config, GameInfo, InputControllers, SaveData, SdlContext, Ui, WindowAudio};

impl Ui {
    pub fn try_new() -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        let audio_subsystem = sdl_context.audio()?;
        let joystick_subsystem = sdl_context.joystick()?;
        let controller_subsystem = sdl_context.game_controller()?;

        let directory = dirs::config_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("p64");
        let file_path = directory.join("input.json");
        let config_map = config::Config::load(&directory)
            .map_err(|error| format!("Unable to load input configuration: {error}"))?;

        let mut ui = Ui {
            config: Config {
                original: serde_json::to_vec(&config_map).map_err(|error| error.to_string())?,
                file_path,
                config: config_map,
            },
            game_info: GameInfo {
                id: String::new(),
                hash: String::new(),
            },
            input_controllers: InputControllers {
                controllers: std::array::from_fn(|_| input::Controllers::default()),
                joystick_subsystem: Some(joystick_subsystem),
                controller_subsystem: Some(controller_subsystem),
            },
            save_data: SaveData {
                save_type: vec![],
                data: storage::SaveData::default(),
            },
            sdl_context: SdlContext {
                sdl_context: Some(sdl_context),
                video_subsystem: Some(video_subsystem),
                audio_subsystem: Some(audio_subsystem),
            },
            window_audio: WindowAudio {
                window: None,
                audio_device: None,
            },
        };

        crate::audio::try_set_frequency(&mut ui, crate::audio_format::DEFAULT_FREQUENCY)?;

        ui.input_controllers.bind(ui.config.input());

        Ok(ui)
    }
}

impl Default for Ui {
    fn default() -> Self {
        Self::try_new().expect("Unable to initialize SDL frontend")
    }
}

pub fn load_saves(ui: &mut Ui) -> std::io::Result<()> {
    let directory = data_dir()
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Save directory unavailable")
        })?
        .join("p64/saves");
    ui.prepare_saves(&directory)
}

impl Drop for Ui {
    fn drop(&mut self) {
        if let Err(error) = self.flush() {
            tracing::error!("Failed to flush saves/configuration: {error}");
        }
    }
}

pub fn write_saves(ui: &mut Ui) -> std::io::Result<()> {
    ui.save_data.data.write_dirty()
}

impl Ui {
    pub fn prepare_saves(&mut self, directory: &std::path::Path) -> std::io::Result<()> {
        self.save_data
            .data
            .prepare_game(directory, &self.game_info.id, &self.game_info.hash)?;
        self.save_data.save_type = get_save_type(&self.game_info.id);
        Ok(())
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        let saves = write_saves(self);
        let original = serde_json::to_vec(&self.config.config)?;
        let config = if original != self.config.original {
            self.config
                .config
                .save(&self.config.file_path)
                .map(|()| self.config.original = original)
        } else {
            Ok(())
        };
        saves.and(config)
    }

    pub fn rebind_input(&mut self) {
        self.input_controllers.controllers = std::array::from_fn(|_| input::Controllers::default());
        self.input_controllers.bind(&self.config.config.input);
    }

    pub fn input_config(&self) -> &config::Input {
        self.config.input()
    }

    pub fn input_config_mut(&mut self) -> &mut config::Input {
        self.config.input_mut()
    }

    pub fn profile_for_channel(&self, channel: usize) -> Option<(&str, &config::InputProfile)> {
        let input = self.input_config();
        let name = input.input_profile_binding.get(channel)?;

        Some((name, input.input_profiles.get(name)?))
    }

    pub fn joystick_subsystem(&self) -> &sdl2::JoystickSubsystem {
        self.input_controllers
            .joystick_subsystem
            .as_ref()
            .expect("SDL joysticks initialized")
    }

    pub fn new() -> Ui {
        Ui::default()
    }
}

impl Config {
    fn input(&self) -> &config::Input {
        &self.config.input
    }

    fn input_mut(&mut self) -> &mut config::Input {
        &mut self.config.input
    }
}
