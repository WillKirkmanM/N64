use super::*;

/// Audio Interface Register Indices
#[repr(u32)]
pub enum AiRegister {
    DramAddr = 0,
    Len = 1,
    Status = 3,
    DacRate = 4,
    RegsCount = 6,
}

/// Audio Interface Status Flags
#[repr(u32)]
pub enum AiStatus {
    Busy = 0x4000_0000,
    Full = 0x8000_0000,
}

pub struct Audio {
    pub registers: Registers,
    pub fifo: [AiDma; 2],
    pub last_read: u64,
    pub delayed_carry: bool,
}

impl Default for Audio {
    fn default() -> Self {
        Self {
            registers: Registers::new()
                .with_size(AiRegister::RegsCount as usize)
                .build(),
            fifo: [AiDma::default(); 2],
            last_read: 0,
            delayed_carry: false,
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
        self.fifo[index].address =
            self.registers[AiRegister::DramAddr as usize] as u64 & rdram_mask;
        self.fifo[index].length = (self.registers[AiRegister::Len as usize] & !7) as u64;
        self.fifo[index].duration = duration;
    }

    pub(super) fn shift_fifo(&mut self) {
        self.fifo[0] = self.fifo[1];
    }

    pub(super) fn get_aligned_dma_length(&self) -> u64 {
        const ALIGNMENT_MASK: u64 = !7;
        self.registers[AiRegister::Len as usize] as u64 & ALIGNMENT_MASK
    }

    pub(super) fn calculate_sample_rate(&self, vi_clock: u64) -> u64 {
        let dac_rate = self.registers[AiRegister::DacRate as usize] as u64;
        vi_clock / (1 + dac_rate)
    }

    pub(super) fn calculate_dma_duration(&self, vi_clock: u64, cpu_clock: u64) -> u64 {
        const BYTES_PER_SAMPLE: u64 = 4;

        let dma_length = self.get_aligned_dma_length();
        let sample_rate = self.calculate_sample_rate(vi_clock);

        dma_length * cpu_clock / (BYTES_PER_SAMPLE * sample_rate)
    }

    pub(super) fn calculate_remaining_fifo_length(
        &self,
        fifo_index: usize,
        cpu_count: u64,
        next_event: Option<Event>,
    ) -> u64 {
        let fifo = &self.fifo[fifo_index];

        if fifo.duration == 0 || next_event.is_none() {
            return 0;
        }

        let time_remaining = next_event.unwrap().count.saturating_sub(cpu_count);
        let bytes_remaining = time_remaining * fifo.length / fifo.duration;

        bytes_remaining & !7 // 8-byte alignment
    }

    pub(super) fn process_dma_transfer(&mut self) {
        const ADDRESS_MASK: u64 = 0x1FFF;
        const ADDRESS_INCREMENT: u64 = 0x2000;

        self.last_read = self.fifo[0].length;

        if self.delayed_carry {
            self.fifo[0].address += ADDRESS_INCREMENT;
        }

        let next_address = self.fifo[0].address + self.fifo[0].length;
        self.delayed_carry = (next_address & ADDRESS_MASK) == 0;
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
            self.delayed_carry = false;
            false
        }
    }

    pub(super) fn read_register(&self, index: usize) -> u32 {
        self.registers[index]
    }

    pub(super) fn write_register(&mut self, index: usize, value: u32, mask: u32) {
        memory::masked_write_32(&mut self.registers[index], value, mask);
    }

    pub(super) fn play_audio_data(&mut self, current_length: u64) -> Option<(usize, u64)> {
        if current_length >= self.last_read {
            return None;
        }

        let processed_bytes = self.fifo[0].length - self.last_read;
        let bytes_to_play = self.last_read - current_length;
        let play_address = (self.fifo[0].address + processed_bytes) as usize;

        self.last_read = current_length;
        Some((play_address, bytes_to_play))
    }

    pub(super) fn handle_dma_event(n64: &mut N64) {
        if let Some((addr, len)) = n64.audio.play_audio_data(0) {
            play_audio(n64, addr, len);
        }
        Audio::complete_audio_dma(n64);
    }

    pub(super) fn calculate_audio_dma_duration(n64: &N64) -> u64 {
        n64.audio
            .calculate_dma_duration(n64.video.clock, n64.cpu.clock_rate)
    }

    pub(super) fn start_audio_dma(n64: &mut N64) {
        n64.audio.process_dma_transfer();

        let current_count = n64.cpu.mmu.regs[MmuRegister::Count as usize];
        let completion_time = current_count + n64.audio.fifo[0].duration;

        create_event(n64, EventType::AI, completion_time, Audio::handle_dma_event);
        schedule_rcp_interrupt(n64, MI_INTR_AI);
    }

    pub(super) fn queue_audio_data(n64: &mut N64) {
        let duration = Audio::calculate_audio_dma_duration(n64);
        let fifo_index = n64.audio.push_to_fifo(duration, RDRAM_MASK as u64);

        if fifo_index == 0 {
            Audio::start_audio_dma(n64);
        }
    }

    pub(super) fn complete_audio_dma(n64: &mut N64) {
        if n64.audio.pop_from_fifo() {
            Audio::start_audio_dma(n64)
        }
    }
}
