use super::WriteCallback;

#[derive(Default, Clone)]
pub struct MemoryAccess {
    pub mask: Option<usize>,
    pub is_little_endian: bool,
    pub is_rdram: bool,
    pub is_mempak: bool,
    pub write_callback: Option<WriteCallback>,
}
