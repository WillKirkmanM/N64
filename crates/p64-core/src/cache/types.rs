use super::*;

#[derive(Copy, Clone)]
pub struct ICache {
    pub valid: bool,
    pub tag: u32,
    pub index: u16,
    pub words: [u32; 8],
    pub instruction: [fn(&mut N64, u32); 8],
}

#[derive(Copy, Clone, Default)]
pub struct DCache {
    pub valid: bool,
    pub dirty: bool,
    pub store_dirty: bool,
    pub tag: u32,
    pub index: u16,
    pub words: [u32; 4],
}

impl Default for ICache {
    fn default() -> Self {
        fn default_instruction(_device: &mut N64, _value: u32) {}

        ICache {
            valid: false,
            tag: 0,
            index: 0,
            words: [0; 8],
            instruction: [default_instruction; 8],
        }
    }
}
