use super::super::{MMU_INSTRUCTIONS, MMU_SECONDARY_INSTRUCTIONS};
use crate::N64;

type InstructionHandler = fn(&mut N64, u32);

pub struct MmuInstructionTables {
    pub instrs: [InstructionHandler; 32],
    pub instrs2: [InstructionHandler; 32],
}

impl Default for MmuInstructionTables {
    fn default() -> Self {
        fn default_instruction(_device: &mut N64, _value: u32) {}
        Self {
            instrs: [default_instruction; 32],
            instrs2: [default_instruction; 32],
        }
    }
}

impl MmuInstructionTables {
    pub(super) fn initialized() -> Self {
        Self {
            instrs: MMU_INSTRUCTIONS,
            instrs2: MMU_SECONDARY_INSTRUCTIONS,
        }
    }
}
