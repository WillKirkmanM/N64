const WIRED_RESET_DELAY: u64 = 2;
const COUNT_WRITE_PAUSE: u64 = 2;

#[derive(Default)]
pub struct MmuClock {
    pub instruction_clock: u64,
    pub pending_wired_reset: Option<(u64, u64)>,
    pub count_write_origin: Option<u64>,
}

impl MmuClock {
    pub(super) fn begin_instruction(&mut self) -> bool {
        self.instruction_clock = self.instruction_clock.wrapping_add(1);
        let reset = self
            .pending_wired_reset
            .is_some_and(|(when, _)| when == self.instruction_clock);

        if reset {
            self.pending_wired_reset = None;
        }

        reset
    }

    pub(super) fn effective_wired(&self, wired: u64) -> u64 {
        self.pending_wired_reset.map_or(wired, |(_, old)| old)
    }

    pub(super) fn defer_wired_reset(&mut self, old: u64) {
        self.pending_wired_reset = Some((self.instruction_clock + WIRED_RESET_DELAY, old));
    }

    pub(super) fn count_cycles(&self, cycles: u64) -> u64 {
        // The scheduler timestamp continues while Count pauses after a write.
        self.count_write_origin.map_or(cycles, |origin| {
            origin
                + cycles
                    .saturating_sub(origin)
                    .saturating_sub(COUNT_WRITE_PAUSE)
        })
    }

    pub(super) fn count_pause_remaining(&self, cycles: u64) -> u64 {
        self.count_write_origin.map_or(0, |origin| {
            COUNT_WRITE_PAUSE.saturating_sub(cycles.saturating_sub(origin))
        })
    }
}
