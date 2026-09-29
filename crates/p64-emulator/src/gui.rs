mod background;
mod controls;
mod roms;
mod settings;
use crate::{
    controllers::{get_controllers, get_input_profiles},
};
use eframe::egui::{self, ScrollArea};
use p64_gui::{
    ui::Ui,
    video::{set_upscaling, set_viewport},
};
use std::{collections::HashMap, path::PathBuf};

mod library;

mod launch;
mod state;
use launch::GameLaunch;
pub use state::P64Gui;
use state::{ControllerState, DragState, RomLibraryState, SettingsState};

pub use crate::config::{GraphicsSettings, Settings};

fn execute<F: std::future::Future<Output = ()> + Send + 'static>(f: F) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("p64-file-dialog".to_owned())
        .spawn(move || futures::executor::block_on(f))
        .map(|_| ())
}

impl Default for P64Gui {
    fn default() -> Self {
        Self::new()
    }
}

impl P64Gui {
    pub fn new() -> Self {
        Self::try_new().expect("Unable to initialize launcher")
    }

    pub fn try_new() -> Result<Self, String> {
        let game_ui = Ui::try_new()?;
        let config = Self::load_config()
            .map_err(|error| format!("Unable to load launcher configuration: {error}"))?;
        let settings = config.settings.unwrap_or_default();

        let assignments = config
            .controller_settings
            .as_ref()
            .and_then(|settings| settings.controller_guids.clone())
            .unwrap_or_else(|| game_ui.input_config().controller_assignment.clone());

        let selected_profile = config
            .controller_settings
            .as_ref()
            .map(|cs| cs.selected_profile.clone())
            .unwrap_or_else(|| game_ui.input_config().input_profile_binding.clone());

        set_viewport(
            settings.graphics.width as i32,
            settings.graphics.height as i32,
        );
        set_upscaling(settings.graphics.upscaling);

        let mut gui = P64Gui {
            dialog: None,
            scan: None,
            session: None,
            status: None,
            controllers: ControllerState {
                assignments,
                configure_profile: false,
                profile_name: String::new(),
                selected_profile,
                selected_controller: [crate::config::UNASSIGNED_CONTROLLER;
                    p64_gui::input::CONTROLLER_COUNT],
                available: get_controllers(&game_ui),
                profiles: get_input_profiles(&game_ui),
            },
            library: RomLibraryState {
                folder: config.rom_folder,
                roms: Vec::new(),
                info: HashMap::new(),
                query: String::new(),
            },
            settings: SettingsState {
                open: false,
                config: settings,
            },
            drag: DragState::default(),
        };
        gui.controllers.resolve_connected();
        gui.start_scan();
        Ok(gui)
    }

    fn launch_options(&self) -> GameLaunch {
        GameLaunch {
            controllers: self.controller_assignments(),
            profiles: self.controllers.selected_profile.clone(),
            graphics: self.settings.config.graphics.clone(),
        }
    }

    fn launch_rom(&mut self, rom_path: PathBuf) {
        if self.session.is_some() {
            self.status = Some("A game or profile session is already active".to_owned());
            return;
        }
        let launch = self.launch_options();
        self.start_session(
            move || {
                launch
                    .run(&rom_path)
                    .map(|()| background::SessionOutcome::GameStopped)
            },
            "Starting game",
        );
    }
}

impl eframe::App for P64Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_background(ctx);
        if self.session.is_some() && ctx.input(|input| input.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.status =
                Some("Close the game or profile window before closing the launcher".to_owned());
        }
        self.handle_drag_and_drop(ctx);

        self.show_profile_config(ctx);
        self.show_settings(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            let available_space = ui.available_size();

            self.show_menu_bar(ui);
            if let Some(status) = &self.status {
                ui.label(status);
            }

            ScrollArea::vertical().show(ui, |ui| {
                if self.drag.active {
                    self.show_drag_drop_overlay(ui, available_space);
                }

                self.show_action_buttons(ui);
                self.show_rom_folder_selector(ui);

                let list_width = available_space.x - 20.0;
                ui.allocate_ui(egui::vec2(list_width, 0.0), |ui| {
                    self.show_rom_list(ui);
                });
            });
        });

        if self.drag.active {
            ctx.request_repaint();
        }
    }
}
