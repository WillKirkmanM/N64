use super::*;

pub(super) fn andi(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rt(opcode) as usize] = n64.cpu.gpr[rs(opcode) as usize] & imm(opcode) as u64
}

pub(super) fn ori(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rt(opcode) as usize] = n64.cpu.gpr[rs(opcode) as usize] | imm(opcode) as u64
}

pub(super) fn xori(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rt(opcode) as usize] = n64.cpu.gpr[rs(opcode) as usize] ^ imm(opcode) as u64
}

pub(super) fn lui(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rt(opcode) as usize] = se32(((imm(opcode) as u32) << 16) as i32)
}

pub(super) fn daddi(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rt(opcode) as usize] =
        n64.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16))
}

pub(super) fn daddiu(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rt(opcode) as usize] =
        n64.cpu.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16))
}

pub(super) fn sll(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] =
        se32(((n64.cpu.gpr[rt(opcode) as usize] as u32) << sa(opcode)) as i32)
}

pub(super) fn srl(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] =
        se32(((n64.cpu.gpr[rt(opcode) as usize] as u32) >> sa(opcode)) as i32)
}

pub(super) fn sra(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] =
        se32(((n64.cpu.gpr[rt(opcode) as usize] as i64) >> sa(opcode)) as i32)
}

pub(super) fn sllv(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] = se32(
        ((n64.cpu.gpr[rt(opcode) as usize] as u32)
            << (n64.cpu.gpr[rs(opcode) as usize] as u32 & 31)) as i32,
    )
}

pub(super) fn srlv(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] = se32(
        ((n64.cpu.gpr[rt(opcode) as usize] as u32)
            >> (n64.cpu.gpr[rs(opcode) as usize] as u32 & 31)) as i32,
    )
}

pub(super) fn srav(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] = se32(
        ((n64.cpu.gpr[rt(opcode) as usize] as i64) >> (n64.cpu.gpr[rs(opcode) as usize] & 31))
            as i32,
    )
}

pub(super) fn dsllv(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] =
        n64.cpu.gpr[rt(opcode) as usize] << (n64.cpu.gpr[rs(opcode) as usize] & 63)
}

pub(super) fn dsrlv(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] =
        n64.cpu.gpr[rt(opcode) as usize] >> (n64.cpu.gpr[rs(opcode) as usize] & 63)
}

pub(super) fn dsrav(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] = ((n64.cpu.gpr[rt(opcode) as usize] as i64)
        >> (n64.cpu.gpr[rs(opcode) as usize] & 63)) as u64
}

pub(super) fn nor(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] =
        !(n64.cpu.gpr[rs(opcode) as usize] | n64.cpu.gpr[rt(opcode) as usize])
}

pub(super) fn slt(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] = ((n64.cpu.gpr[rs(opcode) as usize] as i64)
        < (n64.cpu.gpr[rt(opcode) as usize] as i64)) as u64
}

pub(super) fn sltu(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] =
        (n64.cpu.gpr[rs(opcode) as usize] < n64.cpu.gpr[rt(opcode) as usize]) as u64
}

pub(super) fn dsll(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] = n64.cpu.gpr[rt(opcode) as usize] << sa(opcode)
}

pub(super) fn dsrl(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] = n64.cpu.gpr[rt(opcode) as usize] >> sa(opcode)
}

pub(super) fn dsra(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] =
        ((n64.cpu.gpr[rt(opcode) as usize] as i64) >> sa(opcode)) as u64
}

pub(super) fn dsll32(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] = n64.cpu.gpr[rt(opcode) as usize] << (32 + sa(opcode))
}

pub(super) fn dsrl32(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] = n64.cpu.gpr[rt(opcode) as usize] >> (32 + sa(opcode))
}

pub(super) fn dsra32(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] =
        ((n64.cpu.gpr[rt(opcode) as usize] as i64) >> (32 + sa(opcode))) as u64
}
