use super::{
    events::{create_event, EventType},
    exceptions::{self, check_pending_interrupts},
    memory::AccessSize,
    mmu::{MmuCause, MmuRegister},
    N64,
};
use crate::builder::registers::register_index;
use crate::builder::registers::FlagCommand;

const MI_INIT_MODE_REG: u32 = 0;
const MI_VERSION_REG: u32 = 1;
const MI_INTR_REG: u32 = 2;
const MI_INTR_MASK_REG: u32 = 3;
pub const MI_REGS_COUNT: u32 = 4;

pub const MI_INTR_SP: u32 = 1 << 0;
pub const MI_INTR_SI: u32 = 1 << 1;
pub const MI_INTR_AI: u32 = 1 << 2;
pub const MI_INTR_VI: u32 = 1 << 3;
pub const MI_INTR_PI: u32 = 1 << 4;
pub const MI_INTR_DP: u32 = 1 << 5;

const MI_CLR_SP: u32 = 1 << 0;
const MI_SET_SP: u32 = 1 << 1;
const MI_CLR_SI: u32 = 1 << 2;
const MI_SET_SI: u32 = 1 << 3;
const MI_CLR_AI: u32 = 1 << 4;
const MI_SET_AI: u32 = 1 << 5;
const MI_CLR_VI: u32 = 1 << 6;
const MI_SET_VI: u32 = 1 << 7;
const MI_CLR_PI: u32 = 1 << 8;
const MI_SET_PI: u32 = 1 << 9;
const MI_CLR_DP: u32 = 1 << 10;
const MI_SET_DP: u32 = 1 << 11;

const MI_INIT_MODE: u32 = 1 << 7;
const MI_EBUS_MODE: u32 = 1 << 8;
const MI_RDRAM_MODE: u32 = 1 << 9;

const MI_CLR_INIT: u32 = 1 << 7;
const MI_SET_INIT: u32 = 1 << 8;
const MI_CLR_EBUS: u32 = 1 << 9;
const MI_SET_EBUS: u32 = 1 << 10;
const MI_CLR_DP_INTR: u32 = 1 << 11;
const MI_CLR_RDRAM: u32 = 1 << 12;
const MI_SET_RDRAM: u32 = 1 << 13;

pub const MI_INIT_LENGTH_MASK: u32 = 0b1111111;

const INIT_COMMANDS: [FlagCommand; 3] = [
    FlagCommand::new(MI_CLR_INIT, MI_SET_INIT, MI_INIT_MODE),
    FlagCommand::new(MI_CLR_EBUS, MI_SET_EBUS, MI_EBUS_MODE),
    FlagCommand::new(MI_CLR_RDRAM, MI_SET_RDRAM, MI_RDRAM_MODE),
];
const INTERRUPT_COMMANDS: [FlagCommand; 6] = [
    FlagCommand::new(MI_CLR_SP, MI_SET_SP, MI_INTR_SP),
    FlagCommand::new(MI_CLR_SI, MI_SET_SI, MI_INTR_SI),
    FlagCommand::new(MI_CLR_AI, MI_SET_AI, MI_INTR_AI),
    FlagCommand::new(MI_CLR_VI, MI_SET_VI, MI_INTR_VI),
    FlagCommand::new(MI_CLR_PI, MI_SET_PI, MI_INTR_PI),
    FlagCommand::new(MI_CLR_DP, MI_SET_DP, MI_INTR_DP),
];

mod types;
pub use types::Mips;

fn set_interrupt_line(n64: &mut N64, asserted: bool) {
    n64.update_cp0_register(MmuRegister::Cause, |cause| {
        if asserted {
            cause | MmuCause::IP2 as u64
        } else {
            cause & !(MmuCause::IP2 as u64)
        }
    });
}

pub fn repeat_enabled(n64: &N64) -> bool {
    n64.mips.regs[MI_INIT_MODE_REG as usize] & MI_INIT_MODE != 0
}

pub fn consume_repeat(n64: &mut N64) -> u32 {
    let length = (n64.mips.regs[MI_INIT_MODE_REG as usize] & MI_INIT_LENGTH_MASK) + 1;
    n64.mips.regs[MI_INIT_MODE_REG as usize] &= !MI_INIT_MODE;

    length
}

pub fn read_regs(n64: &mut N64, address: u64, access_size: AccessSize) -> u32 {
    n64.mips.regs.read(n64, address, access_size)
}

pub fn write_regs(n64: &mut N64, address: u64, value: u32, mask: u32) {
    let reg = register_index(address);

    match reg as u32 {
        MI_INIT_MODE_REG => update_init_mode(n64, value & mask, mask),
        MI_INTR_MASK_REG => update_intr_mask(n64, value & mask),
        _ => (), // Version, pending interrupts, and reserved registers are read-only.
    }

    set_interrupt_line(n64, n64.mips.interrupt_pending());

    check_pending_interrupts(n64)
}

fn update_init_mode(n64: &mut N64, w: u32, mask: u32) {
    n64.mips.regs[MI_INIT_MODE_REG as usize] = (n64.mips.regs[MI_INIT_MODE_REG as usize]
        & !(MI_INIT_LENGTH_MASK & mask))
        | (w & MI_INIT_LENGTH_MASK & mask);

    n64.mips.apply_commands(MI_INIT_MODE_REG, w, &INIT_COMMANDS);

    if w & MI_CLR_DP_INTR != 0 {
        clear_rcp_interrupt(n64, MI_INTR_DP);
    }
}

fn update_intr_mask(n64: &mut N64, command: u32) {
    n64.mips
        .apply_commands(MI_INTR_MASK_REG, command, &INTERRUPT_COMMANDS);
}

pub fn clear_rcp_interrupt(n64: &mut N64, interrupt: u32) {
    n64.mips.regs[MI_INTR_REG as usize] &= !interrupt;

    if !n64.mips.interrupt_pending() {
        set_interrupt_line(n64, false);
    }

    check_pending_interrupts(n64)
}

fn handle_interrupt_common(n64: &mut N64, interrupt: u32) {
    n64.mips.regs[MI_INTR_REG as usize] |= interrupt;

    if n64.mips.interrupt_pending() {
        set_interrupt_line(n64, true);
    }
}

pub fn set_rcp_interrupt(n64: &mut N64, interrupt: u32) {
    handle_interrupt_common(n64, interrupt);

    check_pending_interrupts(n64)
}

pub fn schedule_rcp_interrupt(n64: &mut N64, interrupt: u32) {
    handle_interrupt_common(n64, interrupt);

    create_event(
        n64,
        EventType::InterruptCheck,
        n64.cp0_register(MmuRegister::Count),
        exceptions::check_pending_interrupts,
    )
}

#[cfg(test)]
mod tests;
