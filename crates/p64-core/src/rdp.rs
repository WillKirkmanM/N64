use crate::{
    builder::registers::{register_index, FlagCommand},
    events::{create_event, EventType},
    memory, mips,
    mmu::MmuRegister,
    N64,
};
use p64_gui::video;

const ADDRESS_MASK: u32 = 0x00ff_fff8;
const CLOCK_MASK: u32 = 0x00ff_ffff;
const CLOCK_NUMERATOR: i128 = 2;
const CLOCK_DENOMINATOR: i128 = 3;
const CPU_REGISTER_READ_CYCLES: u64 = 20;
const STATUS_COMMANDS: [FlagCommand; 3] = [
    FlagCommand::new(
        DpcStatusWrite::CLR_XBUS_DMEM_DMA.bits(),
        DpcStatusWrite::SET_XBUS_DMEM_DMA.bits(),
        DpcStatus::XBUS_DMEM_DMA.bits(),
    ),
    FlagCommand::new(
        DpcStatusWrite::CLR_FREEZE.bits(),
        DpcStatusWrite::SET_FREEZE.bits(),
        DpcStatus::FREEZE.bits(),
    ),
    FlagCommand::new(
        DpcStatusWrite::CLR_FLUSH.bits(),
        DpcStatusWrite::SET_FLUSH.bits(),
        DpcStatus::FLUSH.bits(),
    ),
];

mod types;
use types::{CommandAction, DpcStatus, DpcStatusWrite};
pub use types::{DpcReg, DpsReg, Rdp};
#[cfg(test)]
mod tests;

pub fn clock(n64: &N64) -> u32 {
    clock_at(n64, n64.cp0_register(MmuRegister::Count))
}

pub(crate) fn clock_at(n64: &N64, cpu_count: u64) -> u32 {
    n64.rdp().clock(cpu_count)
}

pub fn count_changed(n64: &mut N64, old_count: u64, new_count: u64) {
    n64.rdp_mut().rebase_clock(old_count, new_count);
}

pub fn read_regs_dpc(n64: &mut N64, address: u64, _: memory::AccessSize) -> u32 {
    if !n64.rsp().processor.running {
        crate::mmu::add_cycles(n64, CPU_REGISTER_READ_CYCLES);
    }

    let index = register_index(address);

    if index == DpcReg::Clock as usize {
        clock(n64)
    } else {
        n64.rdp().registers.regs_dpc[index]
    }
}

pub fn write_regs_dpc(n64: &mut N64, address: u64, value: u32, mask: u32) {
    let count = n64.cp0_register(MmuRegister::Count);
    let action = n64
        .rdp_mut()
        .write_command(register_index(address), value, mask, count);

    match action {
        CommandAction::None => {}
        CommandAction::Submit => run_rdp(n64),
        CommandAction::ResumeFrozen => {
            run_rdp(n64);
            n64.rdp_mut().processing.wait_frozen = false;
        }
    }
}

fn run_rdp(n64: &mut N64) {
    let (registers, command_memory) = n64.rdp_buffers();
    let cycles = video::process_rdp_list(registers, command_memory);

    if cycles != 0 {
        create_event(
            n64,
            EventType::DP,
            n64.cp0_register(MmuRegister::Count) + cycles,
            rdp_interrupt_event,
        );
    }
}

pub fn read_regs_dps(n64: &mut N64, address: u64, _: memory::AccessSize) -> u32 {
    n64.rdp().registers.regs_dps[register_index(address)]
}

pub fn write_regs_dps(n64: &mut N64, address: u64, value: u32, mask: u32) {
    memory::masked_write_32(
        &mut n64.rdp_mut().registers.regs_dps[register_index(address)],
        value,
        mask,
    );
}

fn rdp_interrupt_event(n64: &mut N64) {
    video::rdp_full_sync();
    n64.rdp_mut().finish_commands();
    mips::set_rcp_interrupt(n64, mips::MI_INTR_DP);
}
