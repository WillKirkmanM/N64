mod execution;
mod instructions;
mod registers;
mod scheduler;

pub use execution::CpuExecutionState;
pub use instructions::CpuInstructionTables;
pub use registers::IntegerRegisters;
pub use scheduler::CpuScheduler;

use crate::{fpu::Fpu, mmu::Mmu, rcp::Rcp};
use tracing::info;

#[derive(Default, PartialEq)]
pub enum State {
    #[default]
    Step,
    Take,
    NotTaken,
    DelaySlotTaken,
    DelaySlotNotTaken,
    Discard,
    Exception,
}

#[derive(Default)]
pub struct BranchState {
    pub state: State,
    pub pc: u64,
}

pub struct Cpu {
    pub mmu: Mmu,
    pub fpu: Fpu,
    pub rcp: Rcp,
    pub branch_state: BranchState,
    pub registers: IntegerRegisters,
    pub execution: CpuExecutionState,
    pub instructions: CpuInstructionTables,
    pub scheduler: CpuScheduler,
}

impl Default for Cpu {
    fn default() -> Self {
        info!("Initialising CPU...");
        let cpu = Self {
            mmu: Mmu::default(),
            fpu: Fpu::default(),
            rcp: Rcp::default(),
            branch_state: BranchState::default(),
            registers: IntegerRegisters::default(),
            execution: CpuExecutionState::default(),
            instructions: CpuInstructionTables::default(),
            scheduler: CpuScheduler::default(),
        };
        info!("CPU has Initialised");

        cpu
    }
}

impl Cpu {
    #[inline]
    pub fn gpr(&self, index: usize) -> u64 {
        self.registers.gpr(index)
    }

    /// Store raw results; retirement restores r0 and preserves load hazards.
    #[inline]
    pub fn set_gpr(&mut self, index: usize, value: u64) {
        self.registers.set_gpr(index, value);
    }
}

#[derive(Clone, Copy)]
pub(super) enum PreparedInstruction {
    Unprepared,
    Decoded,
    Native(crate::recompiler::NativeInstruction),
}

#[derive(Clone, Copy)]
pub(super) struct NativeRetirementBudget {
    pub(super) end_pc: u64,
    pub(super) end_count: u64,
    pub(super) random: u64,
    pub(super) wired: u64,
}

pub(super) enum FetchOutcome {
    Fault,
    NativeBlock,
    Scalar,
}

impl Cpu {
    pub(super) fn retire_control_flow(&mut self) {
        match self.branch_state.state {
            State::Discard => self.execution.pc = self.execution.pc.wrapping_add(8),
            State::DelaySlotTaken => self.execution.pc = self.branch_state.pc,
            State::Exception => {}
            _ => self.execution.pc = self.execution.pc.wrapping_add(4),
        }
        self.branch_state.state = match self.branch_state.state {
            State::Take => State::DelaySlotTaken,
            State::NotTaken => State::DelaySlotNotTaken,
            _ => State::Step,
        };
    }
}
