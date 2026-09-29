use super::super::InstrFn;
use crate::N64;

pub struct CpuInstructionTables {
    pub instrs: [InstrFn; 64],
    pub special_instrs: [InstrFn; 64],
    pub regimm_instrs: [InstrFn; 32],
}

impl Default for CpuInstructionTables {
    fn default() -> Self {
        fn default_instruction(_device: &mut N64, _value: u32) {}

        Self {
            instrs: [default_instruction; 64],
            special_instrs: [default_instruction; 64],
            regimm_instrs: [default_instruction; 32],
        }
    }
}
