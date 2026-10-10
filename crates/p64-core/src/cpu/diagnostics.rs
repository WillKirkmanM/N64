use crate::{N64, mmu::MmuRegister};

#[derive(Default)]
pub(super) struct InstructionTrace {
    interval: u64,
    instruction_count: u64,
    native_start: u64,
    stop_after: Option<u64>,
}

impl InstructionTrace {
    pub fn from_environment() -> Self {
        Self {
            interval: std::env::var("N64_TRACE_INTERVAL")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
            stop_after: std::env::var("N64_STOP_AFTER_INSTRUCTIONS")
                .ok()
                .and_then(|value| value.parse().ok()),
            ..Self::default()
        }
    }

    pub fn before_fetch(&mut self, n64: &mut N64) {
        if self
            .stop_after
            .is_some_and(|limit| self.instruction_count >= limit)
        {
            n64.cpu_mut().execution.running = 0;
            return;
        }
        self.instruction_count += 1;
        if self.interval != 0 && self.instruction_count.is_multiple_of(self.interval) {
            eprintln!(
                "[CPU] instructions={} PC={:#018x} Count={} next_event={}",
                self.instruction_count,
                n64.cpu().execution.pc,
                n64.cp0_register(MmuRegister::Count),
                n64.cpu().scheduler.next_event_count
            );
        }
        self.native_start = n64.execution_engine.stats.native_executions;
    }

    pub fn after_native_block(&mut self, n64: &N64) {
        self.instruction_count +=
            n64.execution_engine.stats.native_executions - self.native_start - 1;
    }
}
