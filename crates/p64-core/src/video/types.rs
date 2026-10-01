use super::VI_REGS_COUNT;
use crate::builder::registers::Registers;

#[path = "host.rs"]
mod host;
#[path = "timing.rs"]
mod timing;
pub use host::VideoHost;
pub use timing::VideoTiming;

const HORIZONTAL_SYNC_MASK: u32 = 0xfff;
const FIELDS_PER_FRAME: u64 = 2;
const VERTICAL_SYNC_MASK: u32 = 0x3ff;
const CURRENT_FIELD_MASK: u32 = 1;

pub enum ViRegister {
    Status = 0,
    Current = 4,
    VSync = 6,
    HSync = 7,
}

pub struct Video {
    pub regs: Registers,
    pub timing: VideoTiming,
    pub host: VideoHost,
}

impl Default for Video {
    fn default() -> Self {
        Self {
            regs: Registers::new().with_size(VI_REGS_COUNT as usize).build(),
            timing: VideoTiming::default(),
            host: VideoHost::default(),
        }
    }
}

impl Video {
    pub(super) fn update_timing(&mut self, cpu_clock: u64) -> f64 {
        let lines = (self.regs[ViRegister::VSync as usize] & VERTICAL_SYNC_MASK) as u64 + 1;
        let horizontal_period =
            (self.regs[ViRegister::HSync as usize] & HORIZONTAL_SYNC_MASK) as u64 + 1;
        let frame_period = lines * horizontal_period;
        let refresh = self.timing.clock as f64 * FIELDS_PER_FRAME as f64 / frame_period as f64;
        self.timing.delay = (cpu_clock as u128 * frame_period as u128
            / (self.timing.clock as u128 * FIELDS_PER_FRAME as u128))
            as u64;
        self.timing.delay = self.timing.delay.max(1);

        refresh
    }

    pub(super) fn update_current_line(&mut self, next_interrupt: Option<u64>, count: u64) {
        if let Some(next_interrupt) = next_interrupt {
            let remaining = next_interrupt.saturating_sub(count);
            let elapsed = self.timing.delay.saturating_sub(remaining);
            let vertical_sync = self.regs[ViRegister::VSync as usize] & VERTICAL_SYNC_MASK;
            let line = if self.timing.delay == 0 {
                0
            } else {
                (elapsed as u128 * (vertical_sync as u128 + 1) / self.timing.delay as u128) as u32
            };
            self.regs[ViRegister::Current as usize] = if line >= vertical_sync {
                line - vertical_sync
            } else {
                line
            };
        }

        let current = &mut self.regs[ViRegister::Current as usize];
        *current = crate::bits::replace_bits(*current, self.timing.field, CURRENT_FIELD_MASK);
    }
}
