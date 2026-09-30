#[derive(Default)]
pub struct Ui {
    pub game_info: GameInfo,
    pub save_data: SaveData,
    pub window_audio: WindowAudio,
}
#[derive(Default)]
pub struct GameInfo {
    pub id: String,
    pub hash: String,
}
#[derive(Default)]
pub struct SaveData {
    pub save_type: Vec<crate::storage::SaveTypes>,
    pub data: crate::storage::SaveData,
}
#[derive(Default)]
pub struct WindowAudio {
    pub audio_device: Option<AudioQueue>,
}
pub struct AudioQueue;
pub struct AudioSpec {
    pub freq: i32,
}
impl AudioQueue {
    pub fn size(&self) -> u32 {
        0
    }

    pub fn spec(&self) -> AudioSpec {
        AudioSpec {
            freq: crate::audio_format::DEFAULT_FREQUENCY as i32,
        }
    }

    pub fn queue_audio(&mut self, _: &[i16]) -> Result<(), String> {
        Ok(())
    }
}

impl Ui {
    pub fn prepare_saves(&mut self, directory: &std::path::Path) -> std::io::Result<()> {
        self.save_data
            .data
            .prepare_game(directory, &self.game_info.id, &self.game_info.hash)?;
        self.save_data.save_type = crate::storage::get_save_type(&self.game_info.id);
        Ok(())
    }

    pub fn try_new() -> Result<Self, String> {
        Ok(Self::default())
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.save_data.data.write_dirty()
    }
}

#[cfg(test)]
mod tests;
