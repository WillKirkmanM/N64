pub struct PifMemory {
    pub rom: [u8; 1984],
    pub ram: [u8; 64],
}

impl Default for PifMemory {
    fn default() -> Self {
        Self {
            rom: [0; 1984],
            ram: [0; 64],
        }
    }
}
