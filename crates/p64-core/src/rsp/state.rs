use super::types::*;

const CPU_CYCLES_PER_RSP_CYCLE_NUMERATOR: u64 = 3;
const CPU_CYCLES_PER_RSP_CYCLE_DENOMINATOR: u64 = 2;

pub struct Rsp {
    pub registers: RegisterState,
    pub memory: MemoryState,
    pub instruction: InstructionState,
    pub processor: ProcessorState,
    pub vector: VectorState,
    pub vector_control: VectorControlState,
    pub accumulator: AccumulatorState,
    pub division: DivisionState,
    pub instruction_tables: InstructionTables,
}

impl Default for Rsp {
    fn default() -> Self {
        let mut rsp = Rsp {
            registers: RegisterState::default(),
            memory: MemoryState::default(),
            instruction: InstructionState::default(),
            processor: ProcessorState::default(),
            vector: VectorState::default(),
            vector_control: VectorControlState::default(),
            accumulator: AccumulatorState::default(),
            division: DivisionState::default(),
            instruction_tables: InstructionTables::default(),
        };

        rsp.registers.regs[SpReg::Status as usize] = 1;

        rsp
    }
}

impl Rsp {
    pub(super) fn begin_execution(&mut self) {
        self.processor.running = true;
        self.processor.broken = false;
        self.processor.cycle_counter = 0;
    }

    pub(super) fn begin_instruction(&mut self) -> Instructions {
        self.instruction.instruction_type = InstructionType::Su;
        self.registers.gpr[0] = 0;
        if matches!(
            self.processor.branch_state.state,
            CpuState::DelaySlotTaken | CpuState::DelaySlotNotTaken
        ) {
            self.processor.pipeline_full = true;
        }

        let instruction = self.instruction.instructions[(self.registers.pc() / 4) as usize];
        self.processor.prepare_instruction(instruction.opcode);
        instruction
    }

    pub(super) fn retire_instruction(&mut self) {
        let delay_slot = matches!(
            self.processor.branch_state.state,
            CpuState::DelaySlotTaken | CpuState::DelaySlotNotTaken
        );
        let redirected = self.registers.advance_pc(&mut self.processor.branch_state);
        let previous_cycles = self.processor.cycle_counter;
        self.processor.record_issue(&mut self.instruction);
        if redirected {
            self.processor.cycle_counter += 1;
        }
        if delay_slot {
            self.processor.pipeline_full = true;
        }
        if self.registers.regs[SpReg::Status as usize] & SpStatus::Sstep as u32 != 0 {
            self.processor.cycle_counter = self.processor.cycle_counter.max(previous_cycles + 1);
            self.processor.pipeline_full = false;
            self.processor.halted = true;
            self.processor.sync_point = true;
        }
        self.processor
            .advance_readiness(self.processor.cycle_counter - previous_cycles);
    }

    pub(super) fn finish_execution(&mut self) -> u64 {
        self.processor.running = false;
        let (cycles, remainder) = self.converted_slice_cycles();
        self.processor.cpu_cycle_remainder = remainder;
        cycles
    }

    pub(super) fn slice_cpu_time(&self) -> u64 {
        self.processor.cpu_slice_start + self.converted_slice_cycles().0
    }

    fn converted_slice_cycles(&self) -> (u64, u64) {
        let cycles = self.processor.cycle_counter * CPU_CYCLES_PER_RSP_CYCLE_NUMERATOR
            + self.processor.cpu_cycle_remainder;
        (
            cycles / CPU_CYCLES_PER_RSP_CYCLE_DENOMINATOR,
            cycles % CPU_CYCLES_PER_RSP_CYCLE_DENOMINATOR,
        )
    }
}
