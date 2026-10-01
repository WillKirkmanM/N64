use crate::{builder::registers::Registers, events::Event, memory};
#[path = "dma.rs"]
mod dma;
pub use dma::{AiDma, AudioDma};

const DMA_ALIGNMENT: u64 = 8;
const BYTES_PER_SAMPLE: u64 = 4;
const ADDRESS_MASK: u64 = 0x1fff;
const ADDRESS_INCREMENT: u64 = 0x2000;
const AI_WRITE_MASKS: [u32; 6] = [0x00ff_fff8, 0x0003_fff8, 1, 0, 0x3fff, 0xf];

#[repr(u32)]
pub enum AiRegister {
    DramAddr = 0,
    Len = 1,
    Control = 2,
    Status = 3,
    DacRate = 4,
    BitRate = 5,
    RegsCount = 6,
}

#[repr(u32)]
pub enum AiStatus {
    Busy = 0x4000_0000,
    Full = 0x8000_0000,
}

pub struct Audio {
    pub registers: Registers,
    pub dma: AudioDma,
}

impl Default for Audio {
    fn default() -> Self {
        Self {
            registers: Registers::new()
                .with_size(AiRegister::RegsCount as usize)
                .build(),
            dma: AudioDma::default(),
        }
    }
}

impl Audio {
    pub(super) fn is_busy(&self) -> bool {
        (self.registers[AiRegister::Status as usize] & AiStatus::Busy as u32) != 0
    }

    pub(super) fn is_full(&self) -> bool {
        (self.registers[AiRegister::Status as usize] & AiStatus::Full as u32) != 0
    }

    pub(super) fn set_status_flag(&mut self, flag: AiStatus) {
        self.registers[AiRegister::Status as usize] |= flag as u32;
    }

    pub(super) fn clear_status_flag(&mut self, flag: AiStatus) {
        self.registers[AiRegister::Status as usize] &= !(flag as u32);
    }

    pub(super) fn update_fifo_entry(&mut self, index: usize, duration: u64, rdram_mask: u64) {
        self.dma.fifo[index].address =
            self.registers[AiRegister::DramAddr as usize] as u64 & rdram_mask;
        self.dma.fifo[index].length = self.get_aligned_dma_length();
        self.dma.fifo[index].duration = duration;
    }

    pub(super) fn shift_fifo(&mut self) {
        self.dma.fifo[0] = self.dma.fifo[1];
    }

    pub(super) fn get_aligned_dma_length(&self) -> u64 {
        crate::bits::align_down(
            self.registers[AiRegister::Len as usize] as u64,
            DMA_ALIGNMENT,
        )
    }

    pub(super) fn calculate_sample_rate(&self, vi_clock: u64) -> u64 {
        vi_clock / (1 + self.registers[AiRegister::DacRate as usize] as u64)
    }

    pub(super) fn calculate_dma_duration(&self, vi_clock: u64, cpu_clock: u64) -> u64 {
        if vi_clock == 0 {
            return 0;
        }
        let divisor = self.registers[AiRegister::DacRate as usize] as u128 + 1;
        let numerator = self.get_aligned_dma_length() as u128 * cpu_clock as u128 * divisor;
        (numerator / (BYTES_PER_SAMPLE as u128 * vi_clock as u128)) as u64
    }

    pub(super) fn calculate_remaining_fifo_length(
        &self,
        fifo_index: usize,
        cpu_count: u64,
        next_event: Option<Event>,
    ) -> u64 {
        let fifo = &self.dma.fifo[fifo_index];

        let Some(next_event) = next_event else {
            return 0;
        };

        if fifo.duration == 0 {
            return 0;
        }

        let time_remaining = next_event.count.saturating_sub(cpu_count);
        let bytes_remaining = time_remaining * fifo.length / fifo.duration;

        crate::bits::align_down(bytes_remaining, DMA_ALIGNMENT)
    }

    pub(super) fn process_dma_transfer(&mut self) {
        self.dma.last_read = self.dma.fifo[0].length;

        if self.dma.delayed_carry {
            self.dma.fifo[0].address += ADDRESS_INCREMENT;
        }

        let next_address = self.dma.fifo[0].address + self.dma.fifo[0].length;
        self.dma.delayed_carry = (next_address & ADDRESS_MASK) == 0;
    }

    pub(super) fn push_to_fifo(&mut self, duration: u64, rdram_mask: u64) -> usize {
        let fifo_index = if self.is_busy() { 1 } else { 0 };

        self.update_fifo_entry(fifo_index, duration, rdram_mask);

        if fifo_index == 1 {
            self.set_status_flag(AiStatus::Full);
        } else {
            self.set_status_flag(AiStatus::Busy);
        }

        fifo_index
    }

    pub(super) fn pop_from_fifo(&mut self) -> bool {
        if self.is_full() {
            self.shift_fifo();
            self.clear_status_flag(AiStatus::Full);

            true
        } else {
            self.clear_status_flag(AiStatus::Busy);
            self.dma.delayed_carry = false;

            false
        }
    }

    pub(super) fn read_register(&self, index: usize) -> u32 {
        self.registers[index]
    }

    pub(super) fn write_register(&mut self, index: usize, value: u32, mask: u32) {
        let Some(&writable) = AI_WRITE_MASKS.get(index) else {
            return;
        };
        memory::masked_write_32(&mut self.registers[index], value, mask & writable);
    }

    pub(super) fn play_audio_data(&mut self, current_length: u64) -> Option<(usize, u64)> {
        if current_length >= self.dma.last_read {
            return None;
        }

        let processed_bytes = self.dma.fifo[0].length - self.dma.last_read;
        let bytes_to_play = self.dma.last_read - current_length;
        let play_address = (self.dma.fifo[0].address + processed_bytes) as usize;

        self.dma.last_read = current_length;

        Some((play_address, bytes_to_play))
    }
}
