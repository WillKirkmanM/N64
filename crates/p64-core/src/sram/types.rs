use super::{FLASHRAM_PAGE_BYTES, FLASHRAM_TYPE_ID, MX29L1100_ID};

#[derive(Clone, PartialEq, Default)]
pub enum FlashramMode {
    ReadArray,
    ReadSiliconId,
    SectorErase,
    ChipErase,
    #[default]
    Status,
    PageProgram,
}

#[derive(Clone)]
pub struct Flashram {
    pub mode: FlashramMode,
    pub status: u32,
    pub erase_page: u16,
    pub page_buf: [u8; FLASHRAM_PAGE_BYTES],
    pub silicon_id: [u32; 2],
}

impl Default for Flashram {
    fn default() -> Self {
        Flashram {
            mode: FlashramMode::ReadArray,
            status: 0,
            erase_page: 0,
            page_buf: [u8::MAX; FLASHRAM_PAGE_BYTES],
            silicon_id: [FLASHRAM_TYPE_ID, MX29L1100_ID],
        }
    }
}
