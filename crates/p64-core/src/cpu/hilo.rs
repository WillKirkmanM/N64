use super::*;

pub(super) fn sync(_device: &mut N64, _opcode: u32) {}

pub(super) fn mfhi(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] = n64.cpu.hi
}

pub(super) fn mthi(n64: &mut N64, opcode: u32) {
    n64.cpu.hi = n64.cpu.gpr[rs(opcode) as usize]
}

pub(super) fn mflo(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[rd(opcode) as usize] = n64.cpu.lo
}

pub(super) fn mtlo(n64: &mut N64, opcode: u32) {
    n64.cpu.lo = n64.cpu.gpr[rs(opcode) as usize]
}

pub(super) fn div(n64: &mut N64, opcode: u32) {
    if (n64.cpu.gpr[rt(opcode) as usize]) as i32 != 0 {
        n64.cpu.lo = se32(
            (n64.cpu.gpr[rs(opcode) as usize] as i32)
                .wrapping_div(n64.cpu.gpr[rt(opcode) as usize] as i32),
        );
        n64.cpu.hi = se32(
            (n64.cpu.gpr[rs(opcode) as usize] as i32)
                .wrapping_rem(n64.cpu.gpr[rt(opcode) as usize] as i32),
        );
    } else {
        if (n64.cpu.gpr[rs(opcode) as usize] as i32) < 0 {
            n64.cpu.lo = 1
        } else {
            n64.cpu.lo = u64::MAX
        }
        n64.cpu.hi = se32(n64.cpu.gpr[rs(opcode) as usize] as i32)
    }

    add_cycles(n64, 36)
}

pub(super) fn divu(n64: &mut N64, opcode: u32) {
    if (n64.cpu.gpr[rt(opcode) as usize]) as u32 != 0 {
        n64.cpu.lo = se32(
            ((n64.cpu.gpr[rs(opcode) as usize] as u32) / (n64.cpu.gpr[rt(opcode) as usize] as u32))
                as i32,
        );
        n64.cpu.hi = se32(
            ((n64.cpu.gpr[rs(opcode) as usize] as u32) % (n64.cpu.gpr[rt(opcode) as usize] as u32))
                as i32,
        );
    } else {
        n64.cpu.lo = u64::MAX;
        n64.cpu.hi = se32(n64.cpu.gpr[rs(opcode) as usize] as u32 as i32)
    }

    add_cycles(n64, 36)
}

pub(super) fn mult(n64: &mut N64, opcode: u32) {
    let result = ((n64.cpu.gpr[rs(opcode) as usize]) as i32 as i64)
        .wrapping_mul(n64.cpu.gpr[rt(opcode) as usize] as i32 as i64);

    n64.cpu.lo = se32((result) as i32);
    n64.cpu.hi = se32((result >> 32) as i32);

    add_cycles(n64, 4);
}

pub(super) fn multu(n64: &mut N64, opcode: u32) {
    let result = (n64.cpu.gpr[rs(opcode) as usize] as u32 as u64)
        .wrapping_mul(n64.cpu.gpr[rt(opcode) as usize] as u32 as u64);

    n64.cpu.lo = se32((result) as u32 as i32);
    n64.cpu.hi = se32((result >> 32) as u32 as i32);

    add_cycles(n64, 4);
}

pub(super) fn dmult(n64: &mut N64, opcode: u32) {
    let result = ((n64.cpu.gpr[rs(opcode) as usize] as i64 as i128)
        * (n64.cpu.gpr[rt(opcode) as usize] as i64 as i128)) as u128;
    n64.cpu.lo = result as u64;
    n64.cpu.hi = (result >> 64) as u64;
    add_cycles(n64, 7)
}

pub(super) fn dmultu(n64: &mut N64, opcode: u32) {
    let result =
        (n64.cpu.gpr[rs(opcode) as usize] as u128) * (n64.cpu.gpr[rt(opcode) as usize] as u128);
    n64.cpu.lo = result as u64;
    n64.cpu.hi = (result >> 64) as u64;
    add_cycles(n64, 7)
}

pub(super) fn ddiv(n64: &mut N64, opcode: u32) {
    if n64.cpu.gpr[rt(opcode) as usize] as i64 != 0 {
        n64.cpu.lo = (((n64.cpu.gpr[rs(opcode) as usize] as i64) as i128)
            / (n64.cpu.gpr[rt(opcode) as usize] as i64) as i128) as u64;
        n64.cpu.hi = (((n64.cpu.gpr[rs(opcode) as usize] as i64) as i128)
            % (n64.cpu.gpr[rt(opcode) as usize] as i64) as i128) as u64;
    } else {
        if ((n64.cpu.gpr[rs(opcode) as usize]) as i64) < 0 {
            n64.cpu.lo = 1
        } else {
            n64.cpu.lo = u64::MAX
        }
        n64.cpu.hi = n64.cpu.gpr[rs(opcode) as usize]
    }
    add_cycles(n64, 68);
}

pub(super) fn ddivu(n64: &mut N64, opcode: u32) {
    if n64.cpu.gpr[rt(opcode) as usize] != 0 {
        n64.cpu.lo = n64.cpu.gpr[rs(opcode) as usize] / n64.cpu.gpr[rt(opcode) as usize];
        n64.cpu.hi = n64.cpu.gpr[rs(opcode) as usize] % n64.cpu.gpr[rt(opcode) as usize]
    } else {
        n64.cpu.lo = u64::MAX;
        n64.cpu.hi = n64.cpu.gpr[rs(opcode) as usize]
    }
    add_cycles(n64, 68);
}
