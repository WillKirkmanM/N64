use super::{imm, rd, rs, rt, sa, se16, CpuState, RegisterState, Rsp, SpReg2, ADDR_MASK};
use crate::{rsp, N64};

const LINK_REGISTER: usize = 31;
const SHIFT_MASK: u32 = 31;

enum ShiftSource {
    Immediate,
    Register,
}

#[inline]
fn binary(registers: &mut RegisterState, opcode: u32, operation: impl FnOnce(u32, u32) -> u32) {
    let left = registers.gpr[rs(opcode) as usize];
    let right = registers.gpr[rt(opcode) as usize];
    registers.gpr[rd(opcode) as usize] = operation(left, right);
}

#[inline]
fn immediate(registers: &mut RegisterState, opcode: u32, operation: impl FnOnce(u32, u16) -> u32) {
    let value = registers.gpr[rs(opcode) as usize];
    registers.gpr[rt(opcode) as usize] = operation(value, imm(opcode));
}

#[inline]
fn shift(
    registers: &mut RegisterState,
    opcode: u32,
    source: ShiftSource,
    operation: impl FnOnce(u32, u32) -> u32,
) {
    let amount = match source {
        ShiftSource::Immediate => sa(opcode),
        ShiftSource::Register => registers.gpr[rs(opcode) as usize] & SHIFT_MASK,
    };
    let value = registers.gpr[rt(opcode) as usize];
    registers.gpr[rd(opcode) as usize] = operation(value, amount);
}

#[inline]
fn load<const BYTES: usize>(rsp: &mut Rsp, opcode: u32, decode: impl FnOnce([u8; BYTES]) -> u32) {
    let address = rsp.registers.scalar_address(opcode);
    rsp.registers.gpr[rt(opcode) as usize] = decode(rsp.memory.read_scalar(address));
}

#[inline]
fn store<const BYTES: usize>(rsp: &mut Rsp, opcode: u32, encode: impl FnOnce(u32) -> [u8; BYTES]) {
    let address = rsp.registers.scalar_address(opcode);
    let value = rsp.registers.gpr[rt(opcode) as usize];
    rsp.memory.write_scalar(address, encode(value));
}

#[inline]
fn branch(rsp: &mut Rsp, opcode: u32, condition: impl FnOnce(u32, u32) -> bool, link: bool) {
    let left = rsp.registers.gpr[rs(opcode) as usize];
    let right = rsp.registers.gpr[rt(opcode) as usize];
    if condition(left, right) {
        rsp.processor.branch_state.state = CpuState::Take;
        rsp.processor.branch_state.pc = rsp.registers.relative_target(opcode);
    } else {
        rsp.processor.branch_state.state = CpuState::NotTaken;
    }
    if link {
        rsp.registers.gpr[LINK_REGISTER] = (rsp.registers.pc() + 8) & ADDR_MASK;
    }
}

pub(super) fn add(n64: &mut N64, opcode: u32) {
    binary(&mut n64.rsp_mut().registers, opcode, u32::wrapping_add);
}

pub(super) fn sub(n64: &mut N64, opcode: u32) {
    binary(&mut n64.rsp_mut().registers, opcode, u32::wrapping_sub);
}

pub(super) fn and(n64: &mut N64, opcode: u32) {
    binary(&mut n64.rsp_mut().registers, opcode, |a, b| a & b);
}

pub(super) fn or(n64: &mut N64, opcode: u32) {
    binary(&mut n64.rsp_mut().registers, opcode, |a, b| a | b);
}

pub(super) fn xor(n64: &mut N64, opcode: u32) {
    binary(&mut n64.rsp_mut().registers, opcode, |a, b| a ^ b);
}

pub(super) fn nor(n64: &mut N64, opcode: u32) {
    binary(&mut n64.rsp_mut().registers, opcode, |a, b| !(a | b));
}

pub(super) fn slt(n64: &mut N64, opcode: u32) {
    binary(&mut n64.rsp_mut().registers, opcode, |a, b| {
        ((a as i32) < (b as i32)) as u32
    });
}

pub(super) fn sltu(n64: &mut N64, opcode: u32) {
    binary(&mut n64.rsp_mut().registers, opcode, |a, b| (a < b) as u32);
}

pub(super) fn addiu(n64: &mut N64, opcode: u32) {
    immediate(&mut n64.rsp_mut().registers, opcode, |a, b| {
        a.wrapping_add(se16(b as i16))
    });
}

pub(super) fn slti(n64: &mut N64, opcode: u32) {
    immediate(&mut n64.rsp_mut().registers, opcode, |a, b| {
        ((a as i32) < (b as i16 as i32)) as u32
    });
}

pub(super) fn sltiu(n64: &mut N64, opcode: u32) {
    immediate(&mut n64.rsp_mut().registers, opcode, |a, b| {
        (a < se16(b as i16)) as u32
    });
}

pub(super) fn andi(n64: &mut N64, opcode: u32) {
    immediate(&mut n64.rsp_mut().registers, opcode, |a, b| {
        a & u32::from(b)
    });
}

pub(super) fn ori(n64: &mut N64, opcode: u32) {
    immediate(&mut n64.rsp_mut().registers, opcode, |a, b| {
        a | u32::from(b)
    });
}

pub(super) fn xori(n64: &mut N64, opcode: u32) {
    immediate(&mut n64.rsp_mut().registers, opcode, |a, b| {
        a ^ u32::from(b)
    });
}

pub(super) fn lui(n64: &mut N64, opcode: u32) {
    n64.rsp_mut().registers.gpr[rt(opcode) as usize] = u32::from(imm(opcode)) << u16::BITS;
}

pub(super) fn lb(n64: &mut N64, opcode: u32) {
    load(n64.rsp_mut(), opcode, |bytes| {
        i8::from_be_bytes(bytes) as u32
    });
}

pub(super) fn lh(n64: &mut N64, opcode: u32) {
    load(n64.rsp_mut(), opcode, |bytes| {
        i16::from_be_bytes(bytes) as u32
    });
}

pub(super) fn lw(n64: &mut N64, opcode: u32) {
    load(n64.rsp_mut(), opcode, u32::from_be_bytes);
}

pub(super) fn lbu(n64: &mut N64, opcode: u32) {
    load(n64.rsp_mut(), opcode, |bytes| {
        u8::from_be_bytes(bytes) as u32
    });
}

pub(super) fn lhu(n64: &mut N64, opcode: u32) {
    load(n64.rsp_mut(), opcode, |bytes| {
        u16::from_be_bytes(bytes) as u32
    });
}

pub(super) fn sb(n64: &mut N64, opcode: u32) {
    store(n64.rsp_mut(), opcode, |value| (value as u8).to_be_bytes());
}

pub(super) fn sh(n64: &mut N64, opcode: u32) {
    store(n64.rsp_mut(), opcode, |value| (value as u16).to_be_bytes());
}

pub(super) fn sw(n64: &mut N64, opcode: u32) {
    store(n64.rsp_mut(), opcode, |value| value.to_be_bytes());
}

pub(super) fn sll(n64: &mut N64, opcode: u32) {
    shift(
        &mut n64.rsp_mut().registers,
        opcode,
        ShiftSource::Immediate,
        |a, b| a << b,
    );
}

pub(super) fn srl(n64: &mut N64, opcode: u32) {
    shift(
        &mut n64.rsp_mut().registers,
        opcode,
        ShiftSource::Immediate,
        |a, b| a >> b,
    );
}

pub(super) fn sra(n64: &mut N64, opcode: u32) {
    shift(
        &mut n64.rsp_mut().registers,
        opcode,
        ShiftSource::Immediate,
        |a, b| ((a as i32) >> b) as u32,
    );
}

pub(super) fn sllv(n64: &mut N64, opcode: u32) {
    shift(
        &mut n64.rsp_mut().registers,
        opcode,
        ShiftSource::Register,
        |a, b| a << b,
    );
}

pub(super) fn srlv(n64: &mut N64, opcode: u32) {
    shift(
        &mut n64.rsp_mut().registers,
        opcode,
        ShiftSource::Register,
        |a, b| a >> b,
    );
}

pub(super) fn srav(n64: &mut N64, opcode: u32) {
    shift(
        &mut n64.rsp_mut().registers,
        opcode,
        ShiftSource::Register,
        |a, b| ((a as i32) >> b) as u32,
    );
}

pub(super) fn beq(n64: &mut N64, opcode: u32) {
    branch(n64.rsp_mut(), opcode, |a, b| a == b, false);
}

pub(super) fn bne(n64: &mut N64, opcode: u32) {
    branch(n64.rsp_mut(), opcode, |a, b| a != b, false);
}

pub(super) fn blez(n64: &mut N64, opcode: u32) {
    branch(n64.rsp_mut(), opcode, |a, _| (a as i32) <= 0, false);
}

pub(super) fn bgtz(n64: &mut N64, opcode: u32) {
    branch(n64.rsp_mut(), opcode, |a, _| (a as i32) > 0, false);
}

pub(super) fn bltz(n64: &mut N64, opcode: u32) {
    branch(n64.rsp_mut(), opcode, |a, _| (a as i32) < 0, false);
}

pub(super) fn bgez(n64: &mut N64, opcode: u32) {
    branch(n64.rsp_mut(), opcode, |a, _| (a as i32) >= 0, false);
}

pub(super) fn bltzal(n64: &mut N64, opcode: u32) {
    branch(n64.rsp_mut(), opcode, |a, _| (a as i32) < 0, true);
}

pub(super) fn bgezal(n64: &mut N64, opcode: u32) {
    branch(n64.rsp_mut(), opcode, |a, _| (a as i32) >= 0, true);
}

pub(super) use add as addu;
pub(super) use addiu as addi;
pub(super) use lw as lwu;
pub(super) use sub as subu;

pub(super) fn j(n64: &mut N64, opcode: u32) {
    if rsp::in_delay_slot_taken(n64) {
        return;
    }
    n64.rsp.processor.branch_state.state = CpuState::Take;
    n64.rsp.processor.branch_state.pc = n64.rsp.registers.jump_target(opcode)
}

pub(super) fn jal(n64: &mut N64, opcode: u32) {
    if rsp::in_delay_slot_taken(n64) {
        n64.rsp.registers.gpr[31] = (n64.rsp.processor.branch_state.pc + 4) & ADDR_MASK
    } else {
        n64.rsp.registers.gpr[31] = (n64.rsp.registers.regs2[SpReg2::Pc as usize] + 8) & ADDR_MASK
    }
    if !rsp::in_delay_slot_taken(n64) {
        n64.rsp.processor.branch_state.state = CpuState::Take;
        n64.rsp.processor.branch_state.pc = n64.rsp.registers.jump_target(opcode)
    }
}
pub(super) fn jr(n64: &mut N64, opcode: u32) {
    if !rsp::in_delay_slot_taken(n64) {
        n64.rsp.processor.branch_state.state = CpuState::Take;
        n64.rsp.processor.branch_state.pc = n64.rsp.registers.gpr[rs(opcode) as usize]
    }
}

pub(super) fn jalr(n64: &mut N64, opcode: u32) {
    let in_delay_slot_taken = rsp::in_delay_slot_taken(n64);

    if !in_delay_slot_taken {
        n64.rsp.processor.branch_state.state = CpuState::Take;
        n64.rsp.processor.branch_state.pc = n64.rsp.registers.gpr[rs(opcode) as usize]
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
    let rsp = n64.rsp_mut();
    rsp.processor.broken = true;
}
