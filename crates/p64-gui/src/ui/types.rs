use super::*;

pub struct Ui {
    pub config: Config,
    pub game_info: GameInfo,
    pub input_controllers: InputControllers,
    pub save_data: SaveData,
    pub sdl_context: SdlContext,
    pub window_audio: WindowAudio,
}

pub struct Config {
    pub file_path: std::path::PathBuf,
    pub config: config::Config,
}

pub struct GameInfo {
    pub id: String,
    pub hash: String,
}

pub struct InputControllers {
    pub controllers: [input::Controllers; 4],
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

impl Default for Ui {
    fn default() -> Self {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let audio_subsystem = sdl_context.audio().unwrap();
        let joystick_subsystem = sdl_context.joystick().unwrap();
        let controller_subsystem = sdl_context.game_controller().unwrap();

        let file_path = dirs::config_dir().unwrap().join("p64").join("config.json");
        let config_file = std::fs::read(file_path.clone());
        let mut config_map = config::Config::new();
        if config_file.is_ok() {
            let result = serde_json::from_slice(config_file.unwrap().as_ref());
            if result.is_ok() {
                config_map = result.unwrap();
            }
        }

        let mut ui = Ui {
            config: Config {
                file_path,
                config: config_map,
            },
            game_info: GameInfo {
                id: String::new(),
                hash: String::new(),
            },
            input_controllers: InputControllers {
                controllers: [
                    input::Controllers::default(),
                    input::Controllers::default(),
                    input::Controllers::default(),
                    input::Controllers::default(),
                ],
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

        let desired_spec = sdl2::audio::AudioSpecDesired {
            freq: Some(44100),
            channels: Some(2),
            samples: None,
        };
        ui.window_audio.audio_device = Some(
            ui.sdl_context
                .audio_subsystem
                .as_mut()
                .unwrap()
                .open_queue::<i16, _>(None, &desired_spec)
                .unwrap(),
        );
        ui.window_audio.audio_device.as_mut().unwrap().resume();

        let joystick_subsystem = ui.input_controllers.joystick_subsystem.as_ref().unwrap();
        let mut taken = [false; 4];
        for i in 0..4 {
            let controller_assignment = &ui.config.config.input.controller_assignment[i];
            if controller_assignment.is_some() {
                let mut joystick_index = u32::MAX;
                let guid = controller_assignment.clone().unwrap();
                for i in 0..joystick_subsystem.num_joysticks().unwrap() {
                    if joystick_subsystem.device_guid(i).unwrap().to_string() == guid
                        && !taken[i as usize]
                    {
                        joystick_index = i;
                        taken[i as usize] = true;
                        break;
                    }
                }
                if joystick_index < u32::MAX {
                    if ui.config.config.input.input_profile_binding[i] == "Default" {
                        let controller_result = ui
                            .sdl_context
                            .sdl_context
                            .as_ref()
                            .unwrap()
                            .game_controller()
                            .unwrap()
                            .open(joystick_index);
                        if controller_result.is_ok() {
                            ui.input_controllers.controllers[i].controller =
                                Some(controller_result.unwrap());
                        }
                    }
                    if ui.input_controllers.controllers[i].controller.is_none() {
                        let joystick_result = ui
                            .sdl_context
                            .sdl_context
                            .as_ref()
                            .unwrap()
                            .joystick()
                            .unwrap()
                            .open(joystick_index);
                        if joystick_result.is_err() {
                            println!("could not connect joystick")
                        } else {
                            ui.input_controllers.controllers[i].joystick =
                                Some(joystick_result.unwrap());
                        }
                    }
                } else {
                    println!("Could not bind assigned controller");
                }
            }
        }

        let id = ui.game_info.id.as_str();
        ui.save_data.save_type = get_save_type(id);

        let base_path = data_dir().unwrap().join("p64/saves");
        create_dir_all(&base_path).expect("could not create save dir");

        let file_names = ["eep", "sra", "fla", "mpk", "romsave"];
        let mut paths = [
            &mut ui.save_data.data.eeprom_path,
            &mut ui.save_data.data.sram_path,
            &mut ui.save_data.data.flash_path,
            &mut ui.save_data.data.mempak_path,
            &mut ui.save_data.data.romsave_path,
        ];

        for (path, &name) in paths.iter_mut().zip(&file_names) {
            path.clone_from(&base_path);
            path.push(format!(
                "{}-{}.{}",
                ui.game_info.id, ui.game_info.hash, name
            ));
        }

        ui
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        write_saves(self);
        write_config(self);
    }
}

impl Ui {
    pub fn new() -> Ui {
        Ui::default()
    }
}
