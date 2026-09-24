use super::*;

pub fn check_relative_idle_loop(n64: &mut N64, opcode: u32) {
    check_idle_loop!(n64, imm(opcode) as i16 == -1);
}

pub(super) fn check_absolute_idle_loop(n64: &mut N64, opcode: u32) {
    check_idle_loop!(
        n64,
        (opcode & 0x3FFFFFF) as u64 == (n64.cpu.pc_phys & 0x0FFFFFFF) >> 2
    );
}

pub(super) fn j(n64: &mut N64, opcode: u32) {
    if cpu::in_delay_slot_taken(n64) {
        return;
    }
    check_absolute_idle_loop(n64, opcode);
    n64.cpu.branch_state.state = State::Take;
    n64.cpu.branch_state.pc =
        (n64.cpu.pc + 4) & 0xFFFFFFFFF0000000 | ((opcode & 0x3FFFFFF) << 2) as u64
}

pub(super) fn jal(n64: &mut N64, opcode: u32) {
    if cpu::in_delay_slot_taken(n64) {
        n64.cpu.gpr[31] = n64.cpu.branch_state.pc + 4
    } else {
        n64.cpu.gpr[31] = n64.cpu.pc + 8
    }
    if !cpu::in_delay_slot_taken(n64) {
        check_absolute_idle_loop(n64, opcode);
        n64.cpu.branch_state.state = State::Take;
        n64.cpu.branch_state.pc =
            (n64.cpu.pc + 4) & 0xFFFFFFFFF0000000 | ((opcode & 0x3FFFFFF) << 2) as u64
    } else if !cpu::in_delay_slot(n64) {
        n64.cpu.branch_state.state = State::NotTaken;
    }
}

pub(super) fn beql(n64: &mut N64, opcode: u32) {
    if n64.cpu.gpr[rs(opcode) as usize] == n64.cpu.gpr[rt(opcode) as usize] {
        check_relative_idle_loop(n64, opcode);
        n64.cpu.branch_state.state = State::Take;
        n64.cpu.branch_state.pc = n64.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        n64.cpu.branch_state.state = State::Discard;
    }
}

pub(super) fn bnel(n64: &mut N64, opcode: u32) {
    if n64.cpu.gpr[rs(opcode) as usize] != n64.cpu.gpr[rt(opcode) as usize] {
        check_relative_idle_loop(n64, opcode);
        n64.cpu.branch_state.state = State::Take;
        n64.cpu.branch_state.pc = n64.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        n64.cpu.branch_state.state = State::Discard;
    }
}

pub(super) fn blezl(n64: &mut N64, opcode: u32) {
    if n64.cpu.gpr[rs(opcode) as usize] as i64 <= 0 {
        check_relative_idle_loop(n64, opcode);
        n64.cpu.branch_state.state = State::Take;
        n64.cpu.branch_state.pc = n64.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        n64.cpu.branch_state.state = State::Discard;
    }
}

pub(super) fn bgtzl(n64: &mut N64, opcode: u32) {
    if n64.cpu.gpr[rs(opcode) as usize] as i64 > 0 {
        check_relative_idle_loop(n64, opcode);
        n64.cpu.branch_state.state = State::Take;
        n64.cpu.branch_state.pc = n64.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
    } else {
        n64.cpu.branch_state.state = State::Discard;
    }
}

pub(super) fn jr(n64: &mut N64, opcode: u32) {
    if !cpu::in_delay_slot_taken(n64) {
        check_relative_idle_loop(n64, opcode);
        n64.cpu.branch_state.state = State::Take;
        n64.cpu.branch_state.pc = n64.cpu.gpr[rs(opcode) as usize];
    } else if !cpu::in_delay_slot(n64) {
        n64.cpu.branch_state.state = State::NotTaken;
    }
}

pub(super) fn jalr(n64: &mut N64, opcode: u32) {
    let in_delay_slot_taken = cpu::in_delay_slot_taken(n64);

    if !in_delay_slot_taken {
        check_relative_idle_loop(n64, opcode);
        n64.cpu.branch_state.state = State::Take;
        n64.cpu.branch_state.pc = n64.cpu.gpr[rs(opcode) as usize];
    } else if !cpu::in_delay_slot(n64) {
        n64.cpu.branch_state.state = State::NotTaken;
    }

    if in_delay_slot_taken {
        n64.cpu.gpr[rd(opcode) as usize] = n64.cpu.branch_state.pc + 4
    } else {
        n64.cpu.gpr[rd(opcode) as usize] = n64.cpu.pc + 8
    }
}
