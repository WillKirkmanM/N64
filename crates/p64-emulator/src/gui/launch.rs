use crate::config::{apply_bindings, GraphicsSettings};
use p64_core::{run_game, N64};
use p64_gui::input::CONTROLLER_COUNT;
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

pub(super) struct SessionLock {
    _file: File,
}
impl SessionLock {
    pub fn acquire(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|error| format!("Unable to open session lock: {error}"))?;
        file.try_lock().map_err(|error| {
            format!("Another game/profile session is active or cannot be locked: {error}")
        })?;
        Ok(Self { _file: file })
    }
    pub fn acquire_default() -> Result<Self, String> {
        let directory =
            dirs::cache_dir().ok_or_else(|| "Cache directory unavailable".to_owned())?;
        Self::acquire(&directory.join("p64/session.lock"))
    }
}

pub(super) struct GameLaunch {
    pub controllers: [Option<String>; CONTROLLER_COUNT],
    pub profiles: [String; CONTROLLER_COUNT],
    pub graphics: GraphicsSettings,
}

impl GameLaunch {
    pub fn run(self, rom_path: &Path) -> Result<(), String> {
        let _session = SessionLock::acquire_default()?;
        let mut device = N64::with_ui(p64_gui::ui::Ui::try_new()?);
        apply_bindings(&mut device.ui, self.controllers, self.profiles);
        p64_gui::video::set_vsync(self.graphics.vsync);
        p64_gui::video::set_upscaling(self.graphics.upscaling);
        let result = run_game(
            rom_path,
            &mut device,
            self.graphics.width,
            self.graphics.height,
            self.graphics.fullscreen,
        );
        let saves = device
            .ui
            .flush()
            .map_err(|error| format!("Failed to save game/configuration: {error}"));
        result.and(saves)
    }
}
