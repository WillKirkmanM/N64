use super::*;

pub(super) fn j(n64: &mut N64, opcode: u32) {
    if rsp::in_delay_slot_taken(n64) {
        return;
    }
    n64.rsp.processor.branch_state.state = CpuState::Take;
    n64.rsp.processor.branch_state.pc = (n64.rsp.registers.regs2[SpReg2::Pc as usize] + 4)
        & 0xF0000000
        | ((opcode & 0x3FFFFFF) << 2)
}

pub(super) fn jal(n64: &mut N64, opcode: u32) {
    if rsp::in_delay_slot_taken(n64) {
        n64.rsp.registers.gpr[31] = (n64.rsp.processor.branch_state.pc + 4) & ADDR_MASK
    } else {
        n64.rsp.registers.gpr[31] = (n64.rsp.registers.regs2[SpReg2::Pc as usize] + 8) & ADDR_MASK
    }
    if !rsp::in_delay_slot_taken(n64) {
        n64.rsp.processor.branch_state.state = CpuState::Take;
        n64.rsp.processor.branch_state.pc = (n64.rsp.registers.regs2[SpReg2::Pc as usize] + 4)
            & 0xF0000000
            | ((opcode & 0x3FFFFFF) << 2)
    } else if !rsp::in_delay_slot(n64) {
        n64.rsp.processor.branch_state.state = CpuState::NotTaken;
    }
}

pub(super) fn lui(n64: &mut N64, opcode: u32) {
    n64.rsp.registers.gpr[rt(opcode) as usize] = (imm(opcode) as u32) << 16
}

pub(super) fn sb(n64: &mut N64, opcode: u32) {
    let address =
        n64.rsp.registers.gpr[rs(opcode) as usize].wrapping_add(imm(opcode) as i16 as i32 as u32);

    n64.rsp.memory.mem[address as usize & ADDR_MASK as usize] =
        (n64.rsp.registers.gpr[rt(opcode) as usize]) as u8;
}

pub(super) fn sh(n64: &mut N64, opcode: u32) {
    let address =
        n64.rsp.registers.gpr[rs(opcode) as usize].wrapping_add(imm(opcode) as i16 as i32 as u32);

    n64.rsp.memory.mem[address as usize & ADDR_MASK as usize] =
        (n64.rsp.registers.gpr[rt(opcode) as usize] >> 8) as u8;
    n64.rsp.memory.mem[(address as usize + 1) & ADDR_MASK as usize] =
        (n64.rsp.registers.gpr[rt(opcode) as usize]) as u8;
}

pub(super) fn sw(n64: &mut N64, opcode: u32) {
    let address =
        n64.rsp.registers.gpr[rs(opcode) as usize].wrapping_add(imm(opcode) as i16 as i32 as u32);

    n64.rsp.memory.mem[address as usize & ADDR_MASK as usize] =
        (n64.rsp.registers.gpr[rt(opcode) as usize] >> 24) as u8;
    n64.rsp.memory.mem[(address as usize + 1) & ADDR_MASK as usize] =
        (n64.rsp.registers.gpr[rt(opcode) as usize] >> 16) as u8;
    n64.rsp.memory.mem[(address as usize + 2) & ADDR_MASK as usize] =
        (n64.rsp.registers.gpr[rt(opcode) as usize] >> 8) as u8;
    n64.rsp.memory.mem[(address as usize + 3) & ADDR_MASK as usize] =
        (n64.rsp.registers.gpr[rt(opcode) as usize]) as u8;
}

pub(super) fn jr(n64: &mut N64, opcode: u32) {
    if !rsp::in_delay_slot_taken(n64) {
        n64.rsp.processor.branch_state.state = CpuState::Take;
        n64.rsp.processor.branch_state.pc = n64.rsp.registers.gpr[rs(opcode) as usize]
    } else if !rsp::in_delay_slot(n64) {
        n64.rsp.processor.branch_state.state = CpuState::NotTaken;
    }
}

pub(super) fn jalr(n64: &mut N64, opcode: u32) {
    let in_delay_slot_taken = rsp::in_delay_slot_taken(n64);

    if !in_delay_slot_taken {
        n64.rsp.processor.branch_state.state = CpuState::Take;
        n64.rsp.processor.branch_state.pc = n64.rsp.registers.gpr[rs(opcode) as usize]
    } else if !rsp::in_delay_slot(n64) {
        n64.rsp.processor.branch_state.state = CpuState::NotTaken;
    }

    if in_delay_slot_taken {
        n64.rsp.registers.gpr[rd(opcode) as usize] =
            (n64.rsp.processor.branch_state.pc + 4) & ADDR_MASK
    } else {
        n64.rsp.registers.gpr[rd(opcode) as usize] =
            (n64.rsp.registers.regs2[SpReg2::Pc as usize] + 8) & ADDR_MASK
    }
}

pub(super) fn break_(n64: &mut N64, _opcode: u32) {
    n64.rsp.processor.broken = true;
}

pub(super) fn slt(n64: &mut N64, opcode: u32) {
    n64.rsp.registers.gpr[rd(opcode) as usize] =
        ((n64.rsp.registers.gpr[rs(opcode) as usize] as i32)
            < (n64.rsp.registers.gpr[rt(opcode) as usize] as i32)) as u32
}

pub(super) fn sltu(n64: &mut N64, opcode: u32) {
    n64.rsp.registers.gpr[rd(opcode) as usize] = (n64.rsp.registers.gpr[rs(opcode) as usize]
        < n64.rsp.registers.gpr[rt(opcode) as usize])
        as u32
}
