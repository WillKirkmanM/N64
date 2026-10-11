mod instruction;
mod registers;
mod state;
mod status;

pub use registers::{MmuCause, MmuMask, MmuRegister, MmuStatus, MmuXContext};
pub use state::{Mmu, MmuClock, MmuInstructionTables, MmuRegisters, MmuTranslations};
pub use status::Status;

use crate::{cpu, events, opcode::Cp0Opcode, N64};

use super::cpu::State;
use super::events::{create_event, EventType};
use super::exceptions::{
    break_exception, check_pending_interrupts, reserved_exception, syscall_exception,
};
use super::fpu;
use super::tlb;

pub use instruction::{MMU_INSTRUCTIONS, MMU_SECONDARY_INSTRUCTIONS};

const COUNT_CYCLE_SHIFT: u32 = 1;
const COMPARE_PERIOD_CYCLES: u64 = (1_u64 << u32::BITS) << COUNT_CYCLE_SHIFT;

pub fn add_cycles(n64: &mut N64, cycles: u64) {
    n64.cp0_mut().add_cycles(cycles)
}

pub fn begin_instruction(n64: &mut N64) {
    n64.cp0_mut().begin_instruction()
}

fn count_cycles(n64: &N64) -> u64 {
    n64.cp0().count_cycles()
}

pub fn syscall(n64: &mut N64, _opcode: u32) {
    add_cycles(n64, 1);

    syscall_exception(n64)
}

pub fn break_(n64: &mut N64, _opcode: u32) {
    add_cycles(n64, 1);

    break_exception(n64)
}

pub fn reserved(n64: &mut N64, _opcode: u32) {
    #[cfg(feature = "accuracy-diagnostics")]
    crate::exceptions::trace_reserved(n64, _opcode);
    add_cycles(n64, 1);

    reserved_exception(n64, 0)
}

fn move_from_cp0(n64: &mut N64, opcode: u32, extend: impl FnOnce(u64) -> u64) {
    let instruction = cpu::Instruction::decode(opcode);
    let value = extend(get_control_registers(n64, instruction.rd as u32));
    n64.cpu_mut().set_gpr(instruction.rt, value);
}

fn mfc0(n64: &mut N64, opcode: u32) {
    move_from_cp0(n64, opcode, |value| cpu::se32(value as u32 as i32));
}

fn dmfc0(n64: &mut N64, opcode: u32) {
    move_from_cp0(n64, opcode, std::convert::identity);
}

fn mtc0(n64: &mut N64, opcode: u32) {
    add_cycles(n64, 1);
    // VR4300 transfers the full GPR even for MTC0; each CP0 register applies its own mask.
    let data = n64.cpu().gpr(cpu::rt(opcode) as usize);

    set_control_registers(n64, cpu::rd(opcode), data)
}

fn dmtc0(n64: &mut N64, opcode: u32) {
    mtc0(n64, opcode)
}

fn tlbr(n64: &mut N64, _opcode: u32) {
    tlb::read(n64, n64.cp0_register(MmuRegister::Index))
}

fn tlbwi(n64: &mut N64, _opcode: u32) {
    tlb::write(n64, n64.cp0_register(MmuRegister::Index))
}

fn tlbwr(n64: &mut N64, _opcode: u32) {
    let random = n64.cp0_register(MmuRegister::Random);

    tlb::write(n64, random)
}

fn tlbp(n64: &mut N64, _opcode: u32) {
    tlb::probe(n64)
}

fn eret(n64: &mut N64, _opcode: u32) {
    add_cycles(n64, 1);
    let (target, flag) = if n64.cp0().status().error_level() {
        (MmuRegister::ErrorEPC, MmuStatus::ERL)
    } else {
        (MmuRegister::EPC, MmuStatus::EXL)
    };

    n64.cpu.execution.pc = n64.cp0_register(target);
    n64.cp0_mut()
        .update_register(MmuRegister::Status, |status| status & !(flag as u64));
    n64.cpu.branch_state.state = State::Exception;
    n64.cpu.registers.llbit = false;

    check_pending_interrupts(n64)
}

fn execute_cp0(n64: &mut N64, opcode: u32) {
    let instruction = n64
        .cp0()
        .instructions
        .instrs2
        .get(cpu::Instruction::decode(opcode).function())
        .copied()
        .unwrap_or(cp0_noop);

    instruction(n64, opcode)
}

pub fn execute_checked_cp0(n64: &mut N64, opcode: u32) {
    if !n64.cp0().status().cp0_usable() {
        crate::exceptions::cop_unusable_exception(n64, 0);
        return;
    }

    let selector = cpu::rs(opcode) as usize;
    if matches!(
        Cp0Opcode::from_encoding(selector),
        Some(Cp0Opcode::Dmfc0 | Cp0Opcode::Dmtc0)
    ) && !n64.cp0().status().wide_instructions_allowed()
    {
        reserved(n64, opcode);
        return;
    }

    let instruction = n64.cp0().instructions.instrs[selector];

    instruction(n64, opcode)
}

fn cp0_noop(_n64: &mut N64, _opcode: u32) {}

fn get_control_registers(n64: &N64, index: u32) -> u64 {
    match index {
        x if x == MmuRegister::Count as u32 => {
            (count_cycles(n64) >> COUNT_CYCLE_SHIFT) & u64::from(u32::MAX)
        }
        x if x == MmuRegister::Random as u32 => n64.cp0_register(MmuRegister::Random),
        7 | 21 | 22 | 23 | 24 | 25 | 31 => n64.cp0().registers.reg_latch,
        _ => n64.cp0().control_register(index),
    }
}

fn set_control_registers(n64: &mut N64, index: u32, mut data: u64) {
    n64.cp0_mut().registers.reg_latch = data;
    match index {
        x if x == MmuRegister::Count as u32 => {
            write_count(n64, (data & u64::from(u32::MAX)) << COUNT_CYCLE_SHIFT);
            return;
        }
        x if x == MmuRegister::Wired as u32 => n64.cp0_mut().defer_wired_reset(),
        x if x == MmuRegister::Compare as u32 => {
            data &= u64::from(u32::MAX);
            schedule_compare(n64, data);
            n64.cp0_mut()
                .update_register(MmuRegister::Cause, |value| value & !(MmuCause::IP7 as u64));
        }
        x if x == MmuRegister::Status as u32
            && data & MmuStatus::FR as u64
                != n64.cp0_register(MmuRegister::Status) & MmuStatus::FR as u64 =>
        {
            fpu::set_fgr_registers(n64, data)
        }
        _ => {}
    }

    n64.cp0_mut().write_masked(index as usize, data);
    crate::exceptions::schedule_pending_interrupts(n64);
}

fn write_count(n64: &mut N64, count: u64) {
    // Other devices retain their remaining time; Compare matches the newly
    // written Count while preserving its two-cycle pause.
    let previous = n64.cp0_register(MmuRegister::Count);
    events::remove_event(n64, EventType::Compare);
    events::translate_events(n64, previous, count);
    crate::rdp::count_changed(n64, previous, count);
    crate::rdram::count_changed(n64, previous, count);
    let cp0 = n64.cp0_mut();
    cp0.set_register(MmuRegister::Count, count);
    cp0.clock.count_write_origin = Some(count);
    schedule_compare(n64, n64.cp0_register(MmuRegister::Compare));
}

fn schedule_compare(n64: &mut N64, compare: u64) {
    let count = n64.cp0_register(MmuRegister::Count);
    let counter = count_cycles(n64);
    let mut ticks = (compare as u32).wrapping_sub((counter >> COUNT_CYCLE_SHIFT) as u32) as u64;

    if ticks == 0 {
        ticks = COMPARE_PERIOD_CYCLES >> COUNT_CYCLE_SHIFT;
    }

    let pause_remaining = n64.cp0().count_pause_remaining();
    create_event(
        n64,
        EventType::Compare,
        count - (counter & 1) + (ticks << COUNT_CYCLE_SHIFT) + pause_remaining,
        compare_event,
    );
}

fn compare_event(n64: &mut N64) {
    n64.cp0_mut()
        .update_register(MmuRegister::Cause, |value| value | MmuCause::IP7 as u64);

    create_event(
        n64,
        EventType::Compare,
        n64.cpu.scheduler.next_event_count + COMPARE_PERIOD_CYCLES,
        compare_event,
    );
    check_pending_interrupts(n64);
}

pub fn write_mmu_registers(n64: &mut N64) {
    n64.cp0_mut().registers.initialize();

    create_event(
        n64,
        EventType::Compare,
        COMPARE_PERIOD_CYCLES,
        compare_event,
    )
}

#[cfg(test)]
mod accuracy_tests;
