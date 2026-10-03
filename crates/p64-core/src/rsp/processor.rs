use super::InstructionState;

#[path = "pipeline.rs"]
mod pipeline;
use pipeline::Cop0Readiness;

#[derive(Default)]
pub struct ProcessorState {
    pub pipeline_full: bool,
    pub running: bool,
    pub branch_state: BranchState,
    pub broken: bool,
    pub halted: bool,
    pub sync_point: bool,
    pub cycle_counter: u64,
    pub(super) cpu_slice_start: u64,
    cop0_readiness: Cop0Readiness,
    pub(super) cpu_cycle_remainder: u64,
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

impl ProcessorState {
    pub(super) fn record_cop0_read(&mut self, register: usize) {
        self.cop0_readiness.record(register);
    }

    pub(super) fn prepare_instruction(&mut self, opcode: u32) {
        let cycles = self.cop0_readiness.prepare(opcode);
        self.cycle_counter += cycles;
        if cycles != 0 {
            self.pipeline_full = false;
        }
    }

    pub(super) fn advance_readiness(&mut self, cycles: u64) {
        self.cop0_readiness.advance(cycles);
    }

    pub(super) fn record_issue(&mut self, instruction: &mut InstructionState) {
        if instruction.instruction_type == instruction.last_instruction_type {
            self.cycle_counter += 1;
            self.pipeline_full = false;
        } else {
            instruction.last_instruction_type = instruction.instruction_type;

            if self.pipeline_full {
                self.cycle_counter += 1;
                self.pipeline_full = false;
            } else {
                self.pipeline_full = true;
            }
        }
    }
}
