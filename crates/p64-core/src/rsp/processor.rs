use super::*;

/// Processor execution state
pub struct ProcessorState {
    pub pipeline_full: bool,
    pub branch_state: BranchState,
    pub broken: bool,
    pub halted: bool,
    pub sync_point: bool,
    pub cycle_counter: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub enum CpuState {
    #[default]
    Step,
    Take,
    NotTaken,
    DelaySlotTaken,
    DelaySlotNotTaken,
}

#[derive(Copy, Clone, Default)]
pub struct BranchState {
    pub state: CpuState,
    pub pc: u32,
}

impl Default for ProcessorState {
    fn default() -> Self {
        Self {
            pipeline_full: false,
            branch_state: BranchState::default(),
            broken: false,
            halted: false,
            sync_point: false,
            cycle_counter: 0,
        }
    }
}

#[path = "pipeline.rs"]
mod pipeline;
use pipeline::Cop0Readiness;
