use crate::N64;

use super::{TICKS_PER_CPU_CYCLE, TICKS_PER_RCP_CYCLE};

// Approximate scanout bandwidth in 64-byte bursts.
const BURST_BYTES: u64 = 64;
const VI_CLOCKS_PER_PIXEL: u64 = 4;
const RDRAM_BYTES_PER_RCP_CYCLE: u64 = 8;
const DEFAULT_HORIZONTAL_CLOCKS: u64 = 1995;
const SCALE_UNIT: u64 = 1024;
const PHYSICAL_ADDRESS_MASK: u64 = 0xff_ffff;
const VI_STATUS: usize = 0;
const VI_ORIGIN: usize = 1;
const VI_WIDTH: usize = 2;
const VI_V_SYNC: usize = 6;
const VI_H_SYNC: usize = 7;
const VI_H_START: usize = 9;
const VI_V_START: usize = 10;
const VI_X_SCALE: usize = 12;
const VI_Y_SCALE: usize = 13;

pub(super) struct Scanout {
    pub(super) period: u64,
    active_start: u64,
    active_end: u64,
    origin: u64,
    stride_bytes: u64,
    source_bytes: u64,
    x_offset_bytes: u64,
    y_scale: u64,
    y_offset: u64,
    vertical_start: u64,
    vertical_end: u64,
    half_lines: u64,
}

impl Scanout {
    pub(super) fn from_machine(n64: &N64) -> Self {
        Self::from_registers(
            &n64.video.regs.regs,
            n64.cpu.execution.clock_rate,
            n64.video.timing.clock,
        )
    }

    fn from_registers(regs: &[u32], cpu_clock: u64, video_clock: u64) -> Self {
        let video_ticks =
            |clocks: u64| clocks * cpu_clock * TICKS_PER_CPU_CYCLE / video_clock.max(1);
        let horizontal = u64::from(regs[VI_H_SYNC] & 0xfff);
        let period = video_ticks(if horizontal == 0 {
            DEFAULT_HORIZONTAL_CLOCKS
        } else {
            horizontal + 1
        })
        .max(1);
        let start_pixel = u64::from((regs[VI_H_START] >> 16) & 0x3ff);
        let end_pixel = u64::from(regs[VI_H_START] & 0x3ff);
        let x_scale = u64::from(regs[VI_X_SCALE] & 0xfff);
        let x_offset = u64::from((regs[VI_X_SCALE] >> 16) & 0xfff);
        let bytes_per_pixel = match regs[VI_STATUS] & 3 {
            2 => 2,
            3 => 4,
            _ => 0,
        };
        let source_pixels = (end_pixel.saturating_sub(start_pixel) * x_scale).div_ceil(SCALE_UNIT);
        Self {
            period,
            active_start: video_ticks(start_pixel * VI_CLOCKS_PER_PIXEL),
            active_end: video_ticks(end_pixel * VI_CLOCKS_PER_PIXEL).min(period),
            origin: u64::from(regs[VI_ORIGIN] & PHYSICAL_ADDRESS_MASK as u32),
            stride_bytes: u64::from(regs[VI_WIDTH] & 0xfff) * bytes_per_pixel,
            source_bytes: source_pixels * bytes_per_pixel,
            x_offset_bytes: x_offset / SCALE_UNIT * bytes_per_pixel,
            y_scale: u64::from(regs[VI_Y_SCALE] & 0xfff),
            y_offset: u64::from((regs[VI_Y_SCALE] >> 16) & 0xfff),
            vertical_start: u64::from((regs[VI_V_START] >> 16) & 0x3ff),
            vertical_end: u64::from(regs[VI_V_START] & 0x3ff),
            half_lines: u64::from(regs[VI_V_SYNC] & 0x3ff) + 1,
        }
    }

    pub(super) fn burst(&self, clock: u64) -> Option<(u64, u64, u64)> {
        if self.stride_bytes == 0
            || self.source_bytes == 0
            || self.active_end <= self.active_start
            || self.vertical_end <= self.vertical_start
        {
            return None;
        }
        let mut line = clock / self.period;
        let mut phase = clock % self.period;
        for _ in 0..self.half_lines {
            let half_line = (line * 2) % self.half_lines;
            if phase >= self.active_start
                && half_line >= self.vertical_start
                && half_line < self.vertical_end
            {
                return Some(self.line_burst(line, half_line, phase));
            }
            line = line.checked_sub(1)?;
            phase = self.period - 1;
        }
        None
    }

    fn line_burst(&self, line: u64, half_line: u64, phase: u64) -> (u64, u64, u64) {
        let duration = self.active_end - self.active_start;
        let elapsed = phase.min(self.active_end - 1) - self.active_start;
        let bursts = self.source_bytes.div_ceil(BURST_BYTES);
        let index = (elapsed * bursts / duration).min(bursts - 1);
        let start = line * self.period + self.active_start + (index * duration).div_ceil(bursts);
        let source_line = ((half_line - self.vertical_start) * self.y_scale + 2 * self.y_offset)
            / (2 * SCALE_UNIT);
        let address = (self.origin
            + source_line * self.stride_bytes
            + self.x_offset_bytes
            + index * BURST_BYTES)
            & PHYSICAL_ADDRESS_MASK;
        let burst_bytes = (self.source_bytes - index * BURST_BYTES).min(BURST_BYTES);
        let transfer = burst_bytes.div_ceil(RDRAM_BYTES_PER_RCP_CYCLE) * TICKS_PER_RCP_CYCLE;
        (start, address, transfer)
    }
}

#[cfg(test)]
mod tests;
