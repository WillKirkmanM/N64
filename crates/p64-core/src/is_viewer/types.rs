use super::BUFFER_BYTES;

/// ISViewer's development cartridge buffer, used by homebrew test ROMs.
pub struct IsViewer {
    pub buffer: Vec<u8>,
    pub transcript: String,
    pub stop_on_completion: bool,
    pub completed: bool,
}

impl Default for IsViewer {
    fn default() -> Self {
        Self {
            buffer: vec![0; BUFFER_BYTES],
            transcript: String::new(),
            stop_on_completion: false,
            completed: false,
        }
    }
}
