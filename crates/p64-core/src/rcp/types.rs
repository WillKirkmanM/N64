use super::dispatch;
use crate::N64;

const MEMORY_REGISTER_COUNT: usize = 32;

pub struct Rcp {
    pub instrs: [fn(&mut N64, u32); 32],
    pub reg_latch: u64,
    pub(super) memory_registers: [u64; MEMORY_REGISTER_COUNT],
}

impl Default for Rcp {
    fn default() -> Self {
        Rcp {
            instrs: std::array::from_fn(dispatch::main),
            reg_latch: 0,
            memory_registers: [0; MEMORY_REGISTER_COUNT],
        }
    }
}
