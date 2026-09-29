use dirs::ensure_directories;
use eframe::{egui::ViewportBuilder, run_native, NativeOptions};
use gui::P64Gui;
use p64_gui::video::enable_4x_upscaling;

pub mod audio;
pub mod config;
pub mod controllers;
pub mod dirs;
pub mod gui;
pub mod rom;

const INITIAL_WINDOW_SIZE: [f32; 2] = [960.0, 720.0];

pub fn main() {
    tracing_subscriber::fmt::init();

    ensure_directories();
    enable_4x_upscaling();

    run_native(
        "p64",
        NativeOptions {
            viewport: ViewportBuilder::default().with_inner_size(INITIAL_WINDOW_SIZE),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(P64Gui::try_new().map_err(std::io::Error::other)?))),
    )
    .unwrap();
}
