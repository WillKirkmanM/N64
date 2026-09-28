use crate::{memory::AccessSize, N64};

pub(super) type RegReadCallback = fn(&N64, u64, AccessSize) -> u32;
pub(super) type RegWriteCallback = fn(&mut N64, u64, u32, u32);

#[derive(Default, Debug)]
pub struct Registers<T = u32> {
    pub size: usize,
    pub regs: Vec<T>,
    pub read_mask: Option<T>,
    pub write_mask: Option<T>,
    pub read_callback: Option<RegReadCallback>,
    pub write_callback: Option<RegWriteCallback>,
}
