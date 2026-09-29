use super::{execute, P64Gui};
use eframe::egui::{self, RichText, ScrollArea};
use std::sync::mpsc;

impl P64Gui {
    pub(super) fn scan_rom_directory(&mut self) {
        self.start_scan();
    }

    pub(super) fn select_rom_folder(&mut self) {
        if self.dialog.is_some() {
            return;
        }
        let (tx, result) = mpsc::channel();
        self.dialog = Some(super::background::PendingDialog {
            kind: super::background::DialogKind::Folder,
            result,
        });
        if let Err(error) = execute(async move {
            let folder = rfd::AsyncFileDialog::new()
                .set_directory(dirs::home_dir().unwrap_or_default())
                .pick_folder()
                .await;
            let _ = tx.send(folder.map(|folder| folder.path().to_path_buf()));
        }) {
            self.dialog = None;
            self.status = Some(format!("Unable to open folder browser: {error}"));
        }
    }

    pub(super) fn show_rom_folder_selector(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);

        egui::Frame::none()
            .fill(ui.style().visuals.widgets.inactive.bg_fill)
            .stroke(ui.style().visuals.widgets.inactive.bg_stroke)
            .inner_margin(10.0)
            .rounding(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("ROM Directory").strong());

                        let folder_text = if let Some(rom_folder) = &self.library.folder {
                            rom_folder.display().to_string()
                        } else {
                            "No folder selected".to_string()
                        };

                        ui.label(folder_text);
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Browse...").clicked() {
                            self.select_rom_folder();
                        }
                    });
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(format!("{} ROMs found", self.library.roms.len()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.weak("Drop ROM files or folders here");
                    });
                });
            });

        ui.add_space(8.0);
    }

    pub(super) fn show_rom_list(&mut self, ui: &mut egui::Ui) {
        ui.add_space(16.0);

        ui.horizontal(|ui| {
            ui.heading("ROM Library");
            ui.add_space(10.0);

            let visible_count = if self.library.query.is_empty() {
                self.library.roms.len()
            } else {
                self.library
                    .roms
                    .iter()
                    .filter(|path| {
                        self.library
                            .info
                            .get(*path)
                            .is_some_and(|info| info.matches(&self.library.query.to_lowercase()))
                    })
                    .count()
            };

            ui.weak(format!(
                "({} games{})",
                visible_count,
                if !self.library.query.is_empty() {
                    format!(" of {}", self.library.roms.len())
                } else {
                    String::new()
                }
            ));
        });

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Search: ");
            let search_response = ui.add_sized(
                [ui.available_width() - 70.0, 24.0],
                egui::TextEdit::singleline(&mut self.library.query)
                    .hint_text("Type to search ROM names..."),
            );

            if !self.library.query.is_empty() && ui.small_button("✖").clicked() {
                self.library.query.clear();
            }

            if ui.input(|i| i.key_pressed(egui::Key::F) && i.modifiers.ctrl) {
                search_response.request_focus();
            }
        });

        ui.add_space(16.0);

        let available_width = ui.available_width();
        let col1_width = available_width * 0.4;
        let col2_width = available_width * 0.4;
        let col3_width = available_width * 0.2;

        let header_rect = ui.max_rect().shrink2(egui::vec2(
            0.0,
            ui.text_style_height(&egui::TextStyle::Body),
        ));
        ui.painter().rect_filled(
            header_rect.intersect(ui.max_rect()),
            0.0,
            ui.style().visuals.faint_bg_color,
        );

        ui.horizontal(|ui| {
            ui.strong(RichText::new("Filename").size(16.0))
                .on_hover_text("Click on a ROM filename to launch it");
            ui.add_space(col1_width - 80.0);

            ui.strong(RichText::new("Internal Name").size(16.0))
                .on_hover_text("Name stored in the ROM header");
            ui.add_space(col2_width - 100.0);

            ui.strong(RichText::new("MD5").size(16.0))
                .on_hover_text("Complete canonical Z64 checksum");
        });

        ui.add_space(8.0);
        ui.separator();

        let mut launch = None;
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height() - 40.0)
            .show(ui, |ui| {
                let search_query_lower = self.library.query.to_lowercase();
                let filtered_roms: Vec<_> = self
                    .library
                    .roms
                    .iter()
                    .filter(|rom_path| {
                        if self.library.query.is_empty() {
                            return true;
                        }

                        self.library
                            .info
                            .get(*rom_path)
                            .is_some_and(|info| info.matches(&search_query_lower))
                    })
                    .collect();

                if filtered_roms.is_empty() && !self.library.roms.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label(
                            RichText::new("No ROMs matching your search")
                                .size(16.0)
                                .weak(),
                        );
                        ui.add_space(10.0);
                        ui.label("Try a different search term or clear the search box");
                        ui.add_space(40.0);
                    });
                    return;
                }

                for (i, rom_path) in filtered_roms.iter().enumerate() {
                    if let Some(info) = self.library.info.get(*rom_path) {
                        let row_rect = ui
                            .max_rect()
                            .shrink2(egui::vec2(0.0, ui.spacing().item_spacing.y));

                        if i % 2 == 0 {
                            ui.painter().rect_filled(
                                row_rect.intersect(ui.max_rect()),
                                0.0,
                                ui.visuals().faint_bg_color,
                            );
                        }

                        ui.horizontal(|ui| {
                            let response = ui.add_sized(
                                [col1_width, 20.0],
                                egui::SelectableLabel::new(
                                    false,
                                    RichText::new(&info.file_name)
                                        .color(ui.visuals().hyperlink_color),
                                ),
                            );

                            if response.clicked() {
                                launch = Some((*rom_path).clone());
                            }

                            ui.add_sized([col2_width, 20.0], egui::Label::new(&info.internal_name))
                                .on_hover_ui(|ui| {
                                    ui.label(format!("Name: {}", info.internal_name));
                                    ui.label(format!("Path: {}", rom_path.display()));
                                });

                            let md5_short = if info.md5.len() > 10 {
                                format!("{}...", &info.md5[0..8])
                            } else {
                                info.md5.clone()
                            };

                            ui.add_sized(
                                [col3_width, 20.0],
                                egui::Label::new(md5_short.to_uppercase()),
                            )
                            .on_hover_text(info.md5.clone().to_uppercase());
                        });

                        ui.add_space(4.0);
                    }
                }

                if self.library.roms.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label(
                            RichText::new("No ROMs found in the selected directory")
                                .size(16.0)
                                .weak(),
                        );
                        ui.add_space(10.0);
                        ui.label("Select a ROM folder using the folder button or drop ROMs here");
                        ui.add_space(40.0);
                    });
                }
            });
        if let Some(path) = launch {
            self.launch_rom(path);
        }
    }
}
