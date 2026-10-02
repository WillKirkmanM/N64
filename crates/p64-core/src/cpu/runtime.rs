use super::*;

pub fn in_delay_slot(n64: &N64) -> bool {
    matches!(
        n64.cpu.branch_state.state,
        State::DelaySlotTaken | State::DelaySlotNotTaken
    )
}

pub(super) fn in_delay_slot_taken(n64: &N64) -> bool {
    matches!(n64.cpu.branch_state.state, State::DelaySlotTaken)
}

pub fn run(n64: &mut N64) {
    n64.cpu.execution.running = 1;
    while n64.cpu.execution.running == 1 {
        n64.cpu.registers.gpr[0] = 0; // gpr 0 is read only
        let (cached, err);
        (n64.cpu.execution.pc_phys, cached, err) =
            memory::translate_address(n64, n64.cpu.execution.pc, memory::AccessType::Read);

        if err {
            continue;
        } // TLB exception}

        if cached {
            cache::icache_fetch(n64, n64.cpu.execution.pc_phys)
        } else {
            let opcode = n64.memory.bus.memory_map_read[(n64.cpu.execution.pc_phys >> 16) as usize](
                n64,
                n64.cpu.execution.pc_phys,
                memory::AccessSize::Word,
            );
            cpu::decode_opcode(n64, opcode)(n64, opcode);
        }

        match n64.cpu.branch_state.state {
            State::Discard => n64.cpu.execution.pc += 8,
            State::DelaySlotTaken => n64.cpu.execution.pc = n64.cpu.branch_state.pc,
            State::Exception => (),
            _ => n64.cpu.execution.pc += 4,
        }

        n64.cpu.branch_state.state = match n64.cpu.branch_state.state {
            State::Take => State::DelaySlotTaken,
            State::NotTaken => State::DelaySlotNotTaken,
            _ => State::Step,
        };

        add_cycles(n64, 1);

        if n64.cpu.mmu.regs[MmuRegister::Count as usize] > n64.cpu.scheduler.next_event_count {
            events::trigger_event(n64)
        }
    }
}
