
use p64_gui::audio;
use crate::events::Event;
use super::builder::registers::Registers;
use super::mmu::MmuRegister;
use super::events::create_event;
use super::events::get_event;
use super::events::EventType;
use super::memory;
use super::memory::AccessSize;
use super::mips::clear_rcp_interrupt;
use super::mips::schedule_rcp_interrupt;
use super::mips::MI_INTR_AI;
use super::rdram::RDRAM_MASK;
use super::N64;

#[derive(Copy, Clone, Default)]
pub struct AiDma {
    pub address: u64,
    pub length: u64,
    pub duration: u64,
}

pub fn handle_ai_register_read(n64: &mut N64, address: u64, _access_size: AccessSize) -> u32 {
    let reg_index = (address & 0xFFFF) >> 2;

    if reg_index as u32 != AiRegister::Len as u32 {
        return n64.audio.read_register(reg_index as usize);
    }

    let cpu_count = n64.cpu.mmu.regs[MmuRegister::Count as usize];
    let next_event = get_event(n64, EventType::AI).copied();

    // Calculate DMA length
    let current_dma_length = n64
        .audio
        .calculate_remaining_fifo_length(0, cpu_count, next_event);

    // Handle audio playback
    if let Some((addr, len)) = n64.audio.play_audio_data(current_dma_length) {
        play_audio(n64, addr, len)
    }

    current_dma_length as u32
}

pub fn handle_ai_register_write(n64: &mut N64, address: u64, value: u32, mask: u32) {
    let reg_index = (address & 0xFFFF) >> 2;

    match reg_index as u32 {
        x if x == AiRegister::Len as u32 => {
            n64.audio.write_register(reg_index as usize, value, mask);
            if n64.audio.read_register(AiRegister::Len as usize) != 0 {
                Audio::queue_audio_data(n64)
            }
        }
        x if x == AiRegister::Status as u32 => clear_rcp_interrupt(n64, MI_INTR_AI),
        x if x == AiRegister::DacRate as u32 => {
            if n64.audio.read_register(reg_index as usize) != value & mask {
                let frequency = n64.video.clock / (1 + (value & mask)) as u64;
                audio::set_frequency(&mut n64.ui, frequency);
            }
            n64.audio.write_register(reg_index as usize, value, mask);
        }
        _ => n64.audio.write_register(reg_index as usize, value, mask),
    }
}

pub fn play_audio(device: &mut N64, dram_addr: usize, length: u64) {
    let audio_device = device.ui.window_audio.audio_device.as_mut().unwrap();
    let length = length as usize / 2;
    let mut primary_buffer: Vec<i16> = vec![0; length];

    for (i, chunk) in primary_buffer.chunks_mut(2).enumerate() {
        let base_addr = dram_addr + i * 4;
        chunk[0] =
            device.rdram.mem[base_addr + 2] as i16 | (device.rdram.mem[base_addr + 3] as i16) << 8;
        chunk[1] =
            device.rdram.mem[base_addr] as i16 | (device.rdram.mem[base_addr + 1] as i16) << 8;
    }

    let audio_queued = audio_device.size() as f64;
    let spec_freq = audio_device.spec().freq as f64;
    let acceptable_latency = spec_freq * 0.2 * 4.0;
    let min_latency = spec_freq * 0.02 * 4.0;

    if audio_queued < min_latency {
        let silence_buffer: Vec<i16> = vec![0; ((min_latency - audio_queued) * 2.0) as usize & !1];
        audio_device.queue_audio(&silence_buffer).unwrap();
    }

    if audio_queued < acceptable_latency {
        audio_device.queue_audio(&primary_buffer).unwrap();
    }
}

mod types;
pub use types::{AiRegister, AiStatus, Audio};
