use super::{execute, library::is_rom, P64Gui};
use crate::config::UNASSIGNED_CONTROLLER;
use eframe::egui::{self, menu, Button, ComboBox, Grid, RichText, Window};
use p64_gui::input::CONTROLLER_COUNT;
use p64_gui::{input::configure_profile, ui::Ui};

impl P64Gui {
    pub(super) fn show_profile_config(&mut self, ctx: &egui::Context) {
        if !self.controllers.configure_profile {
            return;
        }

        Window::new("Configure Input Profile")
            .resizable(false)
            .collapsible(false)
            .min_width(300.0)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .inner_margin(egui::vec2(10.0, 10.0))
                    .rounding(4.0)
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                let name_label =
                                    ui.label(RichText::new("Profile Name:").strong().size(16.0));
                                ui.add_space(8.0);

                                ui.text_edit_singleline(&mut self.controllers.profile_name)
                                    .labelled_by(name_label.id)
                            });

                            ui.add_space(16.0);

                            ui.horizontal(|ui| {
                                if ui
                                    .add_sized(
                                        [120.0, 30.0],
                                        Button::new(RichText::new("Configure Profile").strong()),
                                    )
                                    .clicked()
                                {
                                    self.handle_profile_config();
                                }

                                ui.add_space(8.0);

                                if ui
                                    .add_sized(
                                        [80.0, 30.0],
                                        Button::new(RichText::new("Close").strong()),
                                    )
                                    .clicked()
                                {
                                    self.controllers.configure_profile = false;
                                }
                            });

                            ui.add_space(8.0);
                        });
                    });
            });
    }

    pub(super) fn show_menu_bar(&mut self, ui: &mut egui::Ui) {
        menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open ROM").clicked() {
                    self.handle_open_rom();
                }

                if ui.button("Settings").clicked() {
                    self.settings.open = true;
                }

                if ui.button("Configure Input Profile").clicked() {
                    self.handle_configure_profile();
                }
            });
        });

        ui.separator();
        ui.add_space(16.0);
    }

    pub(super) fn show_action_buttons(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.horizontal_top(|ui| {
                if ui
                    .add_sized([150.0, 30.0], egui::Button::new("Open ROM"))
                    .clicked()
                {
                    self.handle_open_rom();
                }

                ui.add_space(8.0);

                if ui
                    .add_sized([150.0, 30.0], egui::Button::new("Settings"))
                    .clicked()
                {
                    self.settings.open = true;
                }

                ui.add_space(8.0);

                if ui
                    .add_sized([150.0, 30.0], egui::Button::new("Configure Input Profile"))
                    .clicked()
                {
                    self.handle_configure_profile();
                }
            });
        });
    }

    pub(super) fn handle_profile_config(&mut self) {
        if self.session.is_some() {
            self.status = Some("A session is already active".to_owned());
            return;
        }
        let profile_name = self.controllers.profile_name.clone();
        let started = self.start_session(
            move || {
                let _session = super::launch::SessionLock::acquire_default()?;
                let mut game_ui = Ui::try_new()?;
                let configured = configure_profile(&mut game_ui, profile_name.clone())?;
                game_ui.flush().map_err(|error| error.to_string())?;
                Ok(if configured {
                    super::background::SessionOutcome::ProfileSaved(profile_name)
                } else {
                    super::background::SessionOutcome::Cancelled
                })
            },
            "Configuring input profile",
        );
        if started {
            self.controllers.configure_profile = false;
        }
    }

    pub(super) fn handle_drag_and_drop(&mut self, ctx: &egui::Context) {
        self.drag.active = !ctx.input(|input| input.raw.dropped_files.is_empty());
        self.drag.hover_highlight = !ctx.input(|input| input.raw.hovered_files.is_empty());

        for file in ctx.input(|input| input.raw.dropped_files.clone()) {
            let Some(path) = file.path else {
                continue;
            };

            if path.is_file() && is_rom(&path) {
                self.launch_rom(path);
            } else if path.is_dir() {
                self.library.folder = Some(path);
                self.scan_rom_directory();
                self.save_config();
            }
        }
    }

    pub(super) fn show_drag_drop_overlay(&self, ui: &mut egui::Ui, _available_size: egui::Vec2) {
        if !self.drag.hover_highlight {
            return;
        }

        let rect = ui.max_rect();
        ui.painter().rect(
            rect,
            4.0,
            egui::Color32::from_rgba_premultiplied(20, 100, 150, 100),
            egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(100, 200, 255)),
        );

        let text = "Drop ROM file to play\nDrop folder to set as ROM directory";

        let text_rect = rect.shrink(100.0);
        ui.painter().text(
            text_rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(24.0),
            egui::Color32::WHITE,
        );
    }

    pub(super) fn handle_open_rom(&mut self) {
        if self.dialog.is_some() {
            return;
        }
        let (tx, result) = std::sync::mpsc::channel();
        self.dialog = Some(super::background::PendingDialog {
            kind: super::background::DialogKind::Rom,
            result,
        });
        if let Err(error) = execute(async move {
            let file = rfd::AsyncFileDialog::new().pick_file().await;
            let _ = tx.send(file.map(|file| file.path().to_path_buf()));
        }) {
            self.dialog = None;
            self.status = Some(format!("Unable to open ROM browser: {error}"));
        }
    }

    pub(super) fn handle_configure_profile(&mut self) {
        if self.session.is_none() {
            self.controllers.configure_profile = true;
        }
    }

    pub(super) fn show_controller_grid(&mut self, ui: &mut egui::Ui) {
        if ui
            .add_enabled(self.session.is_none(), Button::new("Refresh controllers"))
            .clicked()
        {
            match Ui::try_new() {
                Ok(game_ui) => {
                    self.controllers.available = crate::controllers::get_controllers(&game_ui);
                    self.controllers.resolve_connected();
                }
                Err(error) => self.status = Some(format!("Unable to refresh controllers: {error}")),
            }
        }
        egui::Frame::none()
            .fill(ui.style().visuals.extreme_bg_color)
            .rounding(8.0)
            .inner_margin(egui::vec2(10.0, 10.0))
            .show(ui, |ui| {
                Grid::new("controller_grid")
                    .spacing([20.0, 12.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.heading("Port");
                        ui.heading("Profile");
                        ui.heading("Controller");
                        ui.end_row();

                        for i in 0..CONTROLLER_COUNT {
                            ui.label(
                                RichText::new(format!("{}", i + 1))
                                    .strong()
                                    .color(ui.style().visuals.widgets.active.fg_stroke.color),
                            );

                            let profile_combo =
                                ComboBox::from_id_salt(format!("profile-combo-{}", i))
                                    .selected_text(
                                        RichText::new(&self.controllers.selected_profile[i])
                                            .strong(),
                                    )
                                    .width(150.0);
                            profile_combo.show_ui(ui, |ui| {
                                for profile in &self.controllers.profiles {
                                    ui.selectable_value(
                                        &mut self.controllers.selected_profile[i],
                                        profile.clone(),
                                        profile.clone(),
                                    );
                                }
                            });

                            let controller_text =
                                usize::try_from(self.controllers.selected_controller[i])
                                    .ok()
                                    .and_then(|index| self.controllers.available.get(index))
                                    .map(|device| device.name.as_str())
                                    .unwrap_or(if self.controllers.assignments[i].is_some() {
                                        "Disconnected"
                                    } else {
                                        "None"
                                    });

                            let controller_combo =
                                ComboBox::from_id_salt(format!("controller-combo-{}", i))
                                    .selected_text(RichText::new(controller_text).strong())
                                    .width(200.0);
                            controller_combo.show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(
                                        self.controllers.assignments[i].is_none(),
                                        "None",
                                    )
                                    .clicked()
                                {
                                    self.controllers.select_controller(i, UNASSIGNED_CONTROLLER);
                                }
                                let mut selected = None;
                                for (index, device) in self.controllers.available.iter().enumerate()
                                {
                                    if ui
                                        .selectable_label(
                                            self.controllers.selected_controller[i] == index as i32,
                                            &device.name,
                                        )
                                        .clicked()
                                    {
                                        selected = Some(index as i32);
                                    }
                                }
                                if let Some(index) = selected {
                                    self.controllers.select_controller(i, index);
                                }
                            });
                            ui.end_row();
                        }
                    });
            });
    }
}
