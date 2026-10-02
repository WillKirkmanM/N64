mod scanout;

use scanout::Scanout;

use crate::{
    memory::AccessSize,
    mmu::{self, MmuRegister},
    N64,
};
use super::RiReg;

const BANK_SHIFT: u32 = 20;
const ROW_SHIFT: u32 = 11;
const BANK_COUNT: usize = 8;
const ROW_MASK: u64 = 511;
const REFRESH_ENABLE: u32 = 1 << 17;
const REFRESH_DELAY_MASK: u32 = 0xff;
const REFRESH_DIRTY_SHIFT: u32 = 8;
const TICKS_PER_CPU_CYCLE: u64 = 8;
const TICKS_PER_RCP_CYCLE: u64 = 12;
const TICKS_PER_RAM_CYCLE: u64 = 3;
const CLEAN_ROW_RETRY_RAM_CYCLES: u64 = 22;
const DIRTY_ROW_RESTORE_RAM_CYCLES: u64 = 8;
const CLEAN_ROW_RETRY_TICKS: u64 = CLEAN_ROW_RETRY_RAM_CYCLES * TICKS_PER_RAM_CYCLE;
const UNCACHED_WORD_CYCLES: u64 = 31;
const UNCACHED_DOUBLE_CYCLES: u64 = 32;
const DATA_MISS_CYCLES: u64 = 36;
const INSTRUCTION_MISS_CYCLES: u64 = 40;
const DATA_REPLY_CYCLES: u64 = 33;

#[derive(Default, Clone, Copy)]
struct Bank {
    row: Option<u64>,
    dirty: bool,
}

impl Bank {
    fn open(&mut self, row: u64) -> u64 {
        let retry = if self.row == Some(row) {
            0
        } else {
            CLEAN_ROW_RETRY_TICKS
                + u64::from(self.dirty) * DIRTY_ROW_RESTORE_RAM_CYCLES * TICKS_PER_RAM_CYCLE
        };
        if self.row != Some(row) {
            self.dirty = false;
        }
        self.row = Some(row);
        retry
    }
}

#[derive(Default)]
pub(crate) struct RdramTiming {
    clock_offset: u64,
    banks: [Bank; BANK_COUNT],
    last_video_burst: Option<(u64, u64, u64)>,
    refresh_line: Option<u64>,
    refresh_ticks: u64,
    last_write_clock: Option<u64>,
}

impl RdramTiming {
    pub(crate) fn rebase(&mut self, old_count: u64, new_count: u64) {
        self.clock_offset = self
            .clock_offset
            .wrapping_add(old_count)
            .wrapping_sub(new_count);
    }

    fn clock(&self, count: u64) -> u64 {
        count
            .wrapping_add(self.clock_offset)
            .wrapping_mul(TICKS_PER_CPU_CYCLE)
    }

    pub(crate) fn write(&mut self, address: u64) {
        if let Some(bank) = self.banks.get_mut((address >> BANK_SHIFT) as usize) {
            bank.open((address >> ROW_SHIFT) & ROW_MASK);
            bank.dirty = true;
        }
    }

    fn refresh_wait(&mut self, clock: u64, period: u64, control: u32) -> u64 {
        if control & REFRESH_ENABLE == 0 {
            return 0;
        }
        let line = clock / period;
        if self.refresh_line != Some(line) {
            let shift = if self.banks.iter().any(|bank| bank.dirty) {
                REFRESH_DIRTY_SHIFT
            } else {
                0
            };
            self.refresh_ticks =
                u64::from((control >> shift) & REFRESH_DELAY_MASK) * TICKS_PER_RCP_CYCLE;
            self.refresh_line = Some(line);
            // Refresh restores the open row but leaves it clean (NEC uPD488170L, p.39).
            for bank in &mut self.banks {
                bank.dirty = false;
            }
        }
        self.refresh_ticks.saturating_sub(clock % period)
    }

    fn video_wait(&mut self, clock: u64, scanout: &Scanout) -> u64 {
        let Some((start, address, duration)) = scanout.burst(clock) else {
            return 0;
        };
        if !self
            .last_video_burst
            .is_some_and(|(previous, location, _)| previous == start && location == address)
        {
            let retry = self
                .banks
                .get_mut((address >> BANK_SHIFT) as usize)
                .map_or(0, |bank| bank.open((address >> ROW_SHIFT) & ROW_MASK));
            self.last_video_burst = Some((start, address, start + duration + retry));
        }
        self.last_video_burst
            .map_or(0, |(_, _, end)| end.saturating_sub(clock))
    }
}

pub(super) fn write(n64: &mut N64, address: u64) {
    let count = n64.cp0_register(MmuRegister::Count);
    let clock = n64.rdram_interface.timing.clock(count);
    if n64.rdram_interface.timing.last_write_clock == Some(clock) {
        n64.rdram_interface.timing.write(address);
        return;
    }
    let scanout = Scanout::from_machine(n64);
    let refresh = n64.rdram_interface.regs[RiReg::Refresh as usize];
    let timing = &mut n64.rdram_interface.timing;
    timing.refresh_wait(clock, scanout.period, refresh);
    timing.video_wait(clock, &scanout);
    timing.last_write_clock = Some(clock);
    timing.write(address);
}

pub(super) fn read(n64: &mut N64, address: u64, size: AccessSize) {
    // Cache refills arrive as word callbacks but occupy one RDRAM transaction.
    if !crate::bits::is_aligned(address, size as u64) {
        return;
    }
    let base_ticks = match size {
        AccessSize::Word => UNCACHED_WORD_CYCLES * TICKS_PER_CPU_CYCLE,
        AccessSize::Dword => UNCACHED_DOUBLE_CYCLES * TICKS_PER_CPU_CYCLE,
        AccessSize::Dcache => DATA_MISS_CYCLES * TICKS_PER_CPU_CYCLE - CLEAN_ROW_RETRY_TICKS,
        AccessSize::Icache => INSTRUCTION_MISS_CYCLES * TICKS_PER_CPU_CYCLE - CLEAN_ROW_RETRY_TICKS,
    };
    let scanout = Scanout::from_machine(n64);
    let count = n64.cp0_register(MmuRegister::Count);
    let refresh = n64.rdram_interface.regs[RiReg::Refresh as usize];
    let timing = &mut n64.rdram_interface.timing;
    let clock = timing.clock(count);
    if matches!(size, AccessSize::Icache) {
        // Instruction refills retain the full-line service window.
        timing.refresh_wait(clock, scanout.period, refresh);
        timing.video_wait(clock, &scanout);
        if let Some(bank) = timing.banks.get_mut((address >> BANK_SHIFT) as usize) {
            bank.open((address >> ROW_SHIFT) & ROW_MASK);
        }
        mmu::add_cycles(n64, INSTRUCTION_MISS_CYCLES);
        return;
    }
    // SysAD commands are issued on MClock edges, independently of CP0 Count writes.
    let mut start = clock.next_multiple_of(TICKS_PER_RCP_CYCLE);
    start += timing.refresh_wait(start, scanout.period, refresh);
    start += timing.video_wait(start, &scanout);
    let retry_ticks = timing
        .banks
        .get_mut((address >> BANK_SHIFT) as usize)
        .map_or(0, |bank| bank.open((address >> ROW_SHIFT) & ROW_MASK));
    let transfer_ticks = base_ticks + retry_ticks;
    let transfer_ticks = match size {
        AccessSize::Dcache => transfer_ticks.max(DATA_REPLY_CYCLES * TICKS_PER_CPU_CYCLE),
        _ => transfer_ticks,
    };
    let ticks = start - clock + transfer_ticks;
    mmu::add_cycles(n64, ticks.div_ceil(TICKS_PER_CPU_CYCLE));
}
