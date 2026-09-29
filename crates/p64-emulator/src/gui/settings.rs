use super::P64Gui;
use crate::config::Config;
use dirs::config_dir;
use eframe::egui::{self, RichText, ScrollArea, Window};
use p64_gui::{
    video::{set_upscaling, set_viewport},
};

const RESOLUTIONS: [(u32, u32); 5] = [
    (640, 480),
    (800, 600),
    (1024, 768),
    (1280, 720),
    (1920, 1080),
];
const UPSCALING_FACTORS: [u8; 4] = [1, 2, 4, 8];

impl P64Gui {
    pub(super) fn load_config() -> std::io::Result<Config> {
        let directory = config_dir().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Configuration directory unavailable",
            )
        })?;
        Config::load(&directory.join("p64"))
    }

    pub(super) fn save_config(&mut self) {
        let Some(directory) = config_dir() else {
            return;
        };
        let controller_settings = self.controllers.settings();
        let config = Config {
            rom_folder: self.library.folder.clone(),
            settings: Some(self.settings.config.clone()),
            controller_settings: Some(controller_settings),
        };
        if let Err(error) = config.save(&directory.join("p64")) {
            self.status = Some(format!("Failed to save launcher settings: {error}"));
        }
    }

    pub(super) fn save_settings(&mut self) {
        self.save_config();
    }

    pub(super) fn show_settings(&mut self, ctx: &egui::Context) {
        if !self.settings.open {
            return;
        }

        let mut settings_changed = false;
        let mut controller_settings_changed = false;

        Window::new("Settings")
                .resizable(true)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        egui::CollapsingHeader::new(RichText::new("Controller Settings").heading().strong())
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.add_space(8.0);

                                let old_controllers = self.controllers.assignments.clone();
                                let old_profiles = self.controllers.selected_profile.clone();

                                self.show_controller_grid(ui);

                                if old_controllers != self.controllers.assignments || old_profiles != self.controllers.selected_profile {
                                    controller_settings_changed = true;
                                }

                                ui.add_space(16.0);
                            });

                        egui::CollapsingHeader::new(RichText::new("Graphics Settings").heading().strong())
                            .default_open(true)
                            .show(ui, |ui| {
                                let settings = &mut self.settings.config;
                                ui.add_space(8.0);

                                egui::Frame::none()
                                    .fill(ui.style().visuals.faint_bg_color)
                                    .rounding(8.0)
                                    .inner_margin(8.0)
                                    .show(ui, |ui| {
                                        ui.heading("Display");
                                        ui.add_space(4.0);

                                        ui.horizontal(|ui| {
                                            ui.label("Resolution:");
                                            ui.add_space(8.0);



                                            egui::ComboBox::from_label("")
                                                .selected_text(format!("{}x{}", settings.graphics.width, settings.graphics.height))
                                                .show_ui(ui, |ui| {
                                                    for (width, height) in RESOLUTIONS.iter() {
                                                        let selected = settings.graphics.width == *width && settings.graphics.height == *height;

                                                        if ui.selectable_label(selected, format!("{}x{}", width, height)).clicked() {
                                                            set_viewport(*width as i32, *height as i32);
                                                            settings.graphics.width = *width;
                                                            settings.graphics.height = *height;
                                                            settings_changed = true;
                                                        }
                                                    }
                                                });
                                        });

                                        ui.add_space(8.0);
                                        if ui.checkbox(&mut settings.graphics.fullscreen, "Fullscreen").changed() {
                                            settings_changed = true;
                                        }

                                        ui.add_space(4.0);

                                        if ui.checkbox(&mut settings.graphics.vsync, "VSync").changed() {
                                            settings_changed = true;
                                        }
                                    });

                                ui.add_space(16.0);

                                egui::Frame::none()
                                    .fill(ui.style().visuals.faint_bg_color)
                                    .rounding(8.0)
                                    .inner_margin(8.0)
                                    .show(ui, |ui| {
                                        ui.heading("Graphics Quality");
                                        ui.add_space(8.0);

                                        ui.label("Upscaling Factor:");
                                        ui.add_space(4.0);

                                        ui.horizontal(|ui| {

                                            for (i, &scale) in UPSCALING_FACTORS.iter().enumerate() {
                                                if i > 0 {
                                                    ui.add_space(8.0);
                                                }

                                                if ui.selectable_value(
                                                    &mut settings.graphics.upscaling,
                                                    scale,
                                                    format!("{}×", scale)
                                                ).clicked() {
                                                    set_upscaling(scale);
                                                    settings_changed = true;
                                                }
                                            }
                                        });

                                        ui.add_space(4.0);
                                        ui.label(RichText::new("Higher values improve graphics quality but require more performance").weak().small());
                                    });
                            });

                        ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                            if ui.button("Close").clicked() {
                                if settings_changed || controller_settings_changed {
                                    self.save_settings();
                                }
                                self.settings.open = false;
                            }

                            ui.add_space(4.0);

                            if ui.button("Apply Settings").clicked() {
                                set_viewport(self.settings.config.graphics.width as i32, self.settings.config.graphics.height as i32);
                                set_upscaling(self.settings.config.graphics.upscaling);

                                self.save_all_settings();
                                self.settings.open = false;
                            }
                        });
                    });
                });

        if settings_changed || controller_settings_changed {
            self.save_config();
        }

        if controller_settings_changed {
            self.save_controller_config();
        }
    }

    pub(super) fn save_all_settings(&mut self) {
        self.save_settings();

        self.save_controller_config();
    }

    pub(super) fn save_controller_config(&mut self) {
        if self.session.is_some() {
            return;
        }

        let mut config = match p64_gui::config::Config::load(
            &dirs::config_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join("p64"),
        ) {
            Ok(config) => config,
            Err(error) => {
                self.status = Some(format!("Failed to load input configuration: {error}"));
                return;
            }
        };
        config.input.controller_assignment = self.controller_assignments();
        config.input.input_profile_binding = self.controllers.selected_profile.clone();
        let path = dirs::config_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("p64/input.json");
        if let Err(error) = config.save(&path) {
            self.status = Some(format!("Failed to save input configuration: {error}"));
        }
    }

    pub(super) fn controller_assignments(
        &self,
    ) -> [Option<String>; p64_gui::input::CONTROLLER_COUNT] {
        self.controllers.assignments.clone()
    }
}
