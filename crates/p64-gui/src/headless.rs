mod types;

pub mod ui {
    pub use super::types::{AudioQueue, AudioSpec, GameInfo, SaveData, Ui, WindowAudio};
    pub use crate::storage::get_save_type;
    pub fn load_saves(ui: &mut Ui) -> std::io::Result<()> {
        if !ui.game_info.id.is_empty() && !ui.game_info.hash.is_empty() {
            ui.save_data.save_type = get_save_type(&ui.game_info.id);
        }
        Ok(())
    }
}
pub mod input {
    pub fn get(_: &mut super::ui::Ui, _: usize) -> u32 {
        0
    }
}
pub mod audio {
    pub fn set_frequency(_: &mut super::ui::Ui, _: u64) {}
}
pub mod video {
    pub fn set_unthrottled(_enabled: bool) {}

    pub struct Renderer;

    impl Drop for Renderer {
        fn drop(&mut self) {}
    }

    pub fn set_vsync(_: bool) {}

    /// # Safety
    /// Matches the native renderer's lifetime contract; no memory is accessed.
    pub unsafe fn start_window(
        _: &mut super::ui::Ui,
        _: *mut u8,
        _: usize,
        _: u32,
        _: u32,
        _: bool,
    ) -> Result<Renderer, String> {
        Err("headless frontend has no graphics renderer".to_owned())
    }

    pub fn process_rdp_list(_: &mut [u32; 8], _: &mut [u8]) -> u64 {
        panic!("headless frontend has no RDP renderer")
    }

    pub fn rdp_full_sync() {}

    pub fn set_register(_: u32, _: u32) {}

    pub fn update_screen() -> u8 {
        1
    }
}
