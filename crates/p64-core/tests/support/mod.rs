#![allow(dead_code)]
use p64_core::{aot, cpu, events, N64};

pub const ALU_FUNCTIONS: [u32; 25] = [
    0, 2, 3, 4, 6, 7, 20, 22, 23, 33, 35, 36, 37, 38, 39, 42, 43, 45, 47, 56, 58, 59, 60, 62, 63,
];
pub const ALU_PRIMARIES: [u32; 8] = [9, 10, 11, 12, 13, 14, 15, 25];

pub fn random(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;

    *seed
}

pub fn artifact_for(words: &[u32]) -> (Vec<u8>, Vec<u8>) {
    let mut rom = vec![0; 64];
    rom[..4].copy_from_slice(&[0x80, 0x37, 0x12, 0x40]);
    for word in words {
        rom.extend_from_slice(&word.to_be_bytes());
    }

    let artifact = aot::compile_artifact(&rom).unwrap();

    (rom, artifact)
}

fn stop(n64: &mut N64) {
    n64.cpu.execution.running = 0;
}

pub fn run_until(n64: &mut N64, deadline: u64) {
    events::create_event(n64, events::EventType::InterruptCheck, deadline, stop);
    cpu::run(n64);
}
