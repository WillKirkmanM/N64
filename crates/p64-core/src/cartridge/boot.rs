use super::CicType;

#[derive(Clone, Default)]
pub struct CartridgeBoot {
    pub pal: bool,
    pub cic_type: CicType,
    pub cic_seed: u8,
    pub rdram_size_offset: usize,
}
