use super::super::dispatch;
use crate::N64;

type Handler = fn(&mut N64, u32);
const PRIMARY_COUNT: usize = 32;
const BRANCH_COUNT: usize = 4;
const FUNCTION_COUNT: usize = 64;

pub struct FpuInstructionTables {
    pub instrs: [Handler; PRIMARY_COUNT],
    pub b_instrs: [Handler; BRANCH_COUNT],
    pub s_instrs: [Handler; FUNCTION_COUNT],
    pub d_instrs: [Handler; FUNCTION_COUNT],
    pub w_instrs: [Handler; FUNCTION_COUNT],
    pub l_instrs: [Handler; FUNCTION_COUNT],
}

impl Default for FpuInstructionTables {
    fn default() -> Self {
        Self {
            instrs: std::array::from_fn(dispatch::main),
            b_instrs: std::array::from_fn(dispatch::branch),
            s_instrs: std::array::from_fn(dispatch::single),
            d_instrs: std::array::from_fn(dispatch::double),
            w_instrs: std::array::from_fn(dispatch::word),
            l_instrs: std::array::from_fn(dispatch::long),
        }
    }
}
