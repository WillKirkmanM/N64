const RESET_PC: u64 = 0xFFFFFFFF_BFC00000;

pub struct CpuExecutionState {
    pub pc: u64,
    pub pc_phys: u64,
    pub data_address: u64,
    pub pending_gpr_load: u8,
    pub running: u8,
    pub clock_rate: u64,
}

impl Default for CpuExecutionState {
    fn default() -> Self {
        Self {
            pc: RESET_PC,
            pc_phys: 0,
            data_address: 0,
            pending_gpr_load: 0,
            running: 0,
            clock_rate: 0,
        }
    }
}
