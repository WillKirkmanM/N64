use super::{State, decode_opcode, execute_decoded_instruction, memory_ops::check_alignment, types::FetchOutcome};
use crate::{N64, cache, events, memory, mmu::{MmuRegister, add_cycles}};

#[cfg(feature = "accuracy-diagnostics")]
#[path = "diagnostics.rs"]
mod diagnostics;

const INSTRUCTION_BYTES: u64 = 4;
const RETIRE_CYCLES: u64 = 1;

pub fn in_delay_slot(n64: &N64) -> bool {
    matches!(
        n64.cpu().branch_state.state,
        State::DelaySlotTaken | State::DelaySlotNotTaken
    )
}

pub(super) fn in_delay_slot_taken(n64: &N64) -> bool {
    matches!(n64.cpu().branch_state.state, State::DelaySlotTaken)
}

pub fn run(n64: &mut N64) {
    #[cfg(feature = "accuracy-diagnostics")]
    let mut trace = diagnostics::InstructionTrace::from_environment();
    n64.cpu_mut().execution.running = 1;

    while n64.cpu().execution.running == 1 {
        #[cfg(feature = "accuracy-diagnostics")]
        trace.before_fetch(n64);

        #[cfg(feature = "accuracy-diagnostics")]
        if n64.cpu().execution.running == 0 {
            break;
        }

        match fetch_and_execute(n64) {
            FetchOutcome::Fault => continue,
            FetchOutcome::NativeBlock => {
                #[cfg(feature = "accuracy-diagnostics")]
                trace.after_native_block(n64);
                continue;
            }
            FetchOutcome::Scalar => retire_scalar(n64),
        }
    }
}

fn fetch_and_execute(n64: &mut N64) -> FetchOutcome {
    n64.cpu_mut().set_gpr(0, 0);
    let pc = n64.cpu().execution.pc;

    if !check_alignment(n64, pc, INSTRUCTION_BYTES, memory::AccessType::Read) {
        return fetch_fault(n64);
    }

    let (physical, cached, error) = memory::translate_address(n64, pc, memory::AccessType::Read);
    n64.cpu_mut().execution.pc_phys = physical;
    if error {
        return fetch_fault(n64);
    }

    let physical = memory::reverse_endian_address(n64, physical, INSTRUCTION_BYTES);
    n64.cpu_mut().execution.pc_phys = physical;
    if cached {
        if cache::icache_fetch(n64, physical) {
            return FetchOutcome::NativeBlock;
        }
    } else {
        let read = n64.memory.bus.memory_map_read[(physical >> 16) as usize];
        let opcode = read(n64, physical, memory::AccessSize::Word);
        execute_decoded_instruction(n64, opcode, decode_opcode(n64, opcode));
    }

    FetchOutcome::Scalar
}

fn fetch_fault(n64: &mut N64) -> FetchOutcome {
    n64.cpu_mut().branch_state.state = State::Step;

    FetchOutcome::Fault
}

fn retire_scalar(n64: &mut N64) {
    n64.cpu_mut().retire_control_flow();
    add_cycles(n64, RETIRE_CYCLES);

    while n64.cpu().execution.running != 0
        && n64.cp0_register(MmuRegister::Count) > n64.cpu().scheduler.next_event_count
    {
        events::trigger_event(n64);
        // A VI callback must not restart a completed test run in the same step.
        if n64.is_viewer.stop_on_completion && n64.is_viewer.completed {
            n64.cpu_mut().execution.running = 0;
        }

        if matches!(n64.cpu().branch_state.state, State::Exception) {
            n64.cpu_mut().branch_state.state = State::Step;
        }
    }
}
