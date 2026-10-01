//! Host playback does not affect DMA deadlines or interrupt delivery.

use crate::builder::registers::register_index;
mod types;
pub use types::*;

use super::events::create_event;
use super::events::get_event;
use super::events::EventType;
use super::memory::AccessSize;
use super::mips::clear_rcp_interrupt;
use super::mips::schedule_rcp_interrupt;
use super::mips::MI_INTR_AI;
use super::mmu::MmuRegister;
use super::rdram::RDRAM_MASK;
use super::N64;
use p64_gui::audio;

const MAX_LATENCY_SECONDS: f64 = 0.2;
const MIN_LATENCY_SECONDS: f64 = 0.02;
const BYTES_PER_STEREO_SAMPLE: f64 = 4.0;
const STEREO_CHANNELS: usize = 2;

pub fn handle_ai_register_read(n64: &mut N64, address: u64, _access_size: AccessSize) -> u32 {
    let reg_index = register_index(address);
    if reg_index >= AiRegister::RegsCount as usize {
        return 0;
    }

    if reg_index as u32 != AiRegister::Len as u32 {
        return n64.audio.read_register(reg_index);
    }

    let cpu_count = n64.cp0_register(MmuRegister::Count);
    let next_event = get_event(n64, EventType::AI).copied();

    let current_dma_length = n64
        .audio
        .calculate_remaining_fifo_length(0, cpu_count, next_event);

    if let Some((addr, len)) = n64.audio.play_audio_data(current_dma_length) {
        play_audio(n64, addr, len)
    }

    current_dma_length as u32
}

pub fn handle_ai_register_write(n64: &mut N64, address: u64, value: u32, mask: u32) {
    let reg_index = register_index(address);
    if reg_index >= AiRegister::RegsCount as usize {
        return;
    }

    match reg_index as u32 {
        x if x == AiRegister::Len as u32 => {
            n64.audio.write_register(reg_index, value, mask);

            if n64.audio.read_register(AiRegister::Len as usize) != 0 && !n64.audio.is_full() {
                Audio::queue_audio_data(n64)
            }
        }
        x if x == AiRegister::Status as u32 => clear_rcp_interrupt(n64, MI_INTR_AI),
        x if x == AiRegister::DacRate as u32 => {
            let previous = n64.audio.read_register(reg_index);
            n64.audio.write_register(reg_index, value, mask);
            if n64.audio.read_register(reg_index) != previous {
                let frequency = n64.audio.calculate_sample_rate(n64.video.timing.clock);
                audio::set_frequency(&mut n64.ui, frequency);
            }
        }
        _ => n64.audio.write_register(reg_index, value, mask),
    }
}

pub fn play_audio(device: &mut N64, dram_addr: usize, length: u64) {
    let Some(audio_device) = device.ui.window_audio.audio_device.as_mut() else {
        return;
    };
    let length = length as usize / 4 * STEREO_CHANNELS;
    let mut primary_buffer: Vec<i16> = vec![0; length];

    for (i, chunk) in primary_buffer.chunks_mut(2).enumerate() {
        let base_addr = dram_addr + i * 4;
        let word: [u8; 4] = std::array::from_fn(|offset| {
            device
                .rdram
                .mem
                .data
                .get((base_addr + offset) & RDRAM_MASK)
                .copied()
                .unwrap_or(0)
        });
        chunk.copy_from_slice(&stereo_samples(&word));
    }

    let audio_queued = audio_device.size() as f64;
    let spec_freq = audio_device.spec().freq as f64;
    let acceptable_latency = spec_freq * MAX_LATENCY_SECONDS * BYTES_PER_STEREO_SAMPLE;
    let min_latency = spec_freq * MIN_LATENCY_SECONDS * BYTES_PER_STEREO_SAMPLE;

    if audio_queued < min_latency {
        let silence_buffer = vec![0i16; silence_sample_count(min_latency - audio_queued)];
        if let Err(error) = audio_device.queue_audio(&silence_buffer) {
            tracing::warn!(%error, "Could not queue audio silence");
        }
    }

    if audio_queued < acceptable_latency {
        if let Err(error) = audio_device.queue_audio(&primary_buffer) {
            tracing::warn!(%error, "Could not queue audio samples");
        }
    }
}

fn silence_sample_count(byte_deficit: f64) -> usize {
    let frames = (byte_deficit / BYTES_PER_STEREO_SAMPLE).ceil() as usize;
    frames * STEREO_CHANNELS
}

fn stereo_samples(word: &[u8]) -> [i16; 2] {
    [
        i16::from_le_bytes([word[2], word[3]]),
        i16::from_le_bytes([word[0], word[1]]),
    ]
}

impl Audio {
    pub(super) fn handle_dma_event(n64: &mut N64) {
        if let Some((addr, len)) = n64.audio.play_audio_data(0) {
            play_audio(n64, addr, len);
        }

        Audio::complete_audio_dma(n64);
    }

    pub(super) fn calculate_audio_dma_duration(n64: &N64) -> u64 {
        n64.audio
            .calculate_dma_duration(n64.video.timing.clock, n64.cpu.execution.clock_rate)
    }

    pub(super) fn start_audio_dma(n64: &mut N64) {
        n64.audio.process_dma_transfer();

        let current_count = n64.cp0_register(MmuRegister::Count);
        let completion_time = current_count + n64.audio.dma.fifo[0].duration;

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

#[cfg(test)]
mod tests;
