use crate::builder::registers::register_index;
mod types;
pub use types::*;

use governor::clock::Clock;
use p64_gui::video;

use crate::events;
use crate::mips;

use super::events::create_event;
use super::events::EventType;
use super::memory;
use super::mips::clear_rcp_interrupt;
use super::mmu::MmuRegister;
use super::N64;

pub const VI_REGS_COUNT: u32 = 14;
const PAL_CLOCK: u64 = 49_656_530;
const NTSC_CLOCK: u64 = 48_681_812;
const SERRATE_SHIFT: u32 = 6;
const VI_WRITE_MASKS: [u32; VI_REGS_COUNT as usize] = [
    0xffff,
    0xff_ffff,
    0xfff,
    0x3ff,
    0,
    0x3fff_ffff,
    0x3ff,
    0x1f_0fff,
    0xfff_0fff,
    0x3ff_03ff,
    0x3ff_03ff,
    0x3ff_03ff,
    0xfff_0fff,
    0xfff_0fff,
];

pub fn read_regs(n64: &mut N64, address: u64, _access_size: memory::AccessSize) -> u32 {
    let reg = register_index(address);
    if reg >= VI_REGS_COUNT as usize {
        return 0;
    }

    if reg as u32 == ViRegister::Current as u32 {
        set_current_line(n64)
    }

    n64.video.regs[reg]
}

pub fn write_regs(n64: &mut N64, address: u64, value: u32, mask: u32) {
    let reg = register_index(address);
    let Some(&writable) = VI_WRITE_MASKS.get(reg) else {
        return;
    };
    let mask = mask & writable;

    match reg as u32 {
        x if x == ViRegister::Current as u32 => clear_rcp_interrupt(n64, mips::MI_INTR_VI),
        x if x == ViRegister::VSync as u32 || x == ViRegister::HSync as u32 => {
            let previous = n64.video.regs[reg];
            memory::masked_write_32(&mut n64.video.regs[reg], value, mask);
            if n64.video.regs[reg] != previous {
                set_expected_refresh_rate(n64);
                if reg as u32 == ViRegister::VSync as u32 {
                    set_vertical_interrupt(n64);
                }

                video::set_register(reg as u32, n64.video.regs[reg]);
            }
        }
        _ => {
            memory::masked_write_32(&mut n64.video.regs[reg], value, mask);
            video::set_register(reg as u32, n64.video.regs[reg]);
        }
    }
}

pub fn set_cpu_clock_speed(n64: &mut N64) {
    n64.video.timing.clock = if n64.cartridge.boot.pal {
        PAL_CLOCK
    } else {
        NTSC_CLOCK
    }
}

fn set_expected_refresh_rate(n64: &mut N64) {
    let cpu_clock = n64.cpu().execution.clock_rate;
    let expected_refresh_rate = n64.video.update_timing(cpu_clock);

    let quota = governor::Quota::with_period(std::time::Duration::from_secs_f64(
        1.0 / expected_refresh_rate,
    ))
    .unwrap();

    n64.video.host.limiter = Some(governor::RateLimiter::direct(quota))
}

fn set_vertical_interrupt(n64: &mut N64) {
    if events::get_event(n64, EventType::VI).is_none() {
        create_event(
            n64,
            EventType::VI,
            n64.cp0_register(MmuRegister::Count) + n64.video.timing.delay,
            vertical_interrupt_event,
        )
    }
}

fn set_current_line(n64: &mut N64) {
    let next_vi = events::get_event(n64, EventType::VI).map(|event| event.count);
    let count = n64.cp0_register(MmuRegister::Count);
    n64.video.update_current_line(next_vi, count);
}

fn vertical_interrupt_event(n64: &mut N64) {
    n64.cpu.execution.running = video::update_screen();
    speed_limiter(n64);
    n64.video.timing.field ^= (n64.video.regs[ViRegister::Status as usize] >> SERRATE_SHIFT) & 1;
    mips::set_rcp_interrupt(n64, mips::MI_INTR_VI);

    create_event(
        n64,
        EventType::VI,
        n64.cpu.scheduler.next_event_count + n64.video.timing.delay,
        vertical_interrupt_event,
    )
}

fn speed_limiter(n64: &N64) {
    if !n64.video.host.host_throttle {
        return;
    }

    let limiter = n64
        .video
        .host
        .limiter
        .as_ref()
        .expect("initialized VI limiter");

    if let Err(outcome) = limiter.check() {
        let duration = outcome.wait_time_from(governor::clock::DefaultClock::default().now());
        std::thread::sleep(duration);
        limiter.check().unwrap();
    }
}

#[cfg(test)]
mod tests;
