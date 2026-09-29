use crate::ui::Ui;

mod atomic;
pub use atomic::atomic_write;
mod document;
mod persistence;
mod save_types;
pub use save_types::get_save_type;
mod types;
pub use types::{SaveData, SaveFile, SaveTypes};

pub fn write_save(ui: &Ui, save_type: SaveTypes) {
    if let Err(error) = ui.save_data.data.write(save_type) {
        tracing::error!("Failed to write save: {error}");
    }
}
