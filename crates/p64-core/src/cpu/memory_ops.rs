use super::*;

pub(super) fn ldl(n64: &mut N64, opcode: u32) {
    let addr = n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16));
    let n = addr & 7;
    let shift = 8 * n;

    let mask = bits_below_mask(8 * n);

    let (mut phys_address, cached, err) =
        memory::translate_address(n64, addr, memory::AccessType::Read);
    if err {
        return;
    }
    phys_address &= !7;

    let mut w = [0; 2];
    w[0] = memory::read_data(n64, phys_address, memory::AccessSize::Dword, cached);
    w[1] = memory::read_data(n64, phys_address + 4, memory::AccessSize::Dword, cached);
    n64.cpu.registers.gpr[rt(opcode) as usize] = (n64.cpu.registers.gpr[rt(opcode) as usize] & mask)
        | (((((w[0]) as u64) << 32) | (w[1]) as u64) << shift)
}

pub(super) fn ldr(n64: &mut N64, opcode: u32) {
    let addr = n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16));
    let n = addr & 7;
    let shift = 8 * (7 - n);

    let mask = if n == 7 {
        0
    } else {
        bits_above_mask(8 * (n + 1))
    };

    let (mut phys_address, cached, err) =
        memory::translate_address(n64, addr, memory::AccessType::Read);
    if err {
        return;
    }
    phys_address &= !7;

    let mut w = [0; 2];
    w[0] = memory::read_data(n64, phys_address, memory::AccessSize::Dword, cached);
    w[1] = memory::read_data(n64, phys_address + 4, memory::AccessSize::Dword, cached);
    n64.cpu.registers.gpr[rt(opcode) as usize] = (n64.cpu.registers.gpr[rt(opcode) as usize] & mask)
        | (((((w[0]) as u64) << 32) | (w[1]) as u64) >> shift)
}

pub(super) fn lb(n64: &mut N64, opcode: u32) {
    let (phys_address, cached, err) = memory::translate_address(
        n64,
        n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        memory::AccessType::Read,
    );
    if err {
        return;
    }

    let shift = bshift(phys_address);
    n64.cpu.registers.gpr[rt(opcode) as usize] =
        se8(
            (memory::read_data(n64, phys_address & !0x3, memory::AccessSize::Word, cached) >> shift)
                as u8 as i8,
        )
}

pub(super) fn lh(n64: &mut N64, opcode: u32) {
    let (phys_address, cached, err) = memory::translate_address(
        n64,
        n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        memory::AccessType::Read,
    );
    if err {
        return;
    }

    let shift = hshift(phys_address);
    n64.cpu.registers.gpr[rt(opcode) as usize] = se16(
        (memory::read_data(n64, phys_address & !0x3, memory::AccessSize::Word, cached) >> shift)
            as u16 as i16,
    )
}

pub(super) fn lwl(n64: &mut N64, opcode: u32) {
    let addr = n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16));
    let n = addr & 3;
    let shift = 8 * n;

    let mask = bits_below_mask(8 * n) as u32;

    let (mut phys_address, cached, err) =
        memory::translate_address(n64, addr, memory::AccessType::Read);
    if err {
        return;
    }
    phys_address &= !3;

    n64.cpu.registers.gpr[rt(opcode) as usize] = se32(
        ((n64.cpu.registers.gpr[rt(opcode) as usize] as u32) & mask
            | memory::read_data(n64, phys_address, memory::AccessSize::Word, cached) << shift)
            as i32,
    )
}

pub(super) fn lw(n64: &mut N64, opcode: u32) {
    let (phys_address, cached, err) = memory::translate_address(
        n64,
        n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        memory::AccessType::Read,
    );
    if err {
        return;
    }

    n64.cpu.registers.gpr[rt(opcode) as usize] =
        se32(memory::read_data(n64, phys_address, memory::AccessSize::Word, cached) as i32)
}

pub(super) fn lbu(n64: &mut N64, opcode: u32) {
    let (phys_address, cached, err) = memory::translate_address(
        n64,
        n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        memory::AccessType::Read,
    );
    if err {
        return;
    }

    let shift = bshift(phys_address);
    n64.cpu.registers.gpr[rt(opcode) as usize] =
        (memory::read_data(n64, phys_address & !0x3, memory::AccessSize::Word, cached) >> shift)
            as u8 as u64
}

pub(super) fn lhu(n64: &mut N64, opcode: u32) {
    let (phys_address, cached, err) = memory::translate_address(
        n64,
        n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        memory::AccessType::Read,
    );
    if err {
        return;
    }

    let shift = hshift(phys_address);
    n64.cpu.registers.gpr[rt(opcode) as usize] =
        (memory::read_data(n64, phys_address & !0x3, memory::AccessSize::Word, cached) >> shift)
            as u16 as u64
}

pub(super) fn lwr(n64: &mut N64, opcode: u32) {
    let addr = n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16));
    let n = addr & 3;
    let shift = 8 * (3 - n);

    let mask = if n == 3 {
        0
    } else {
        bits_above_mask(8 * (n + 1))
    };

    let (mut phys_address, cached, err) =
        memory::translate_address(n64, addr, memory::AccessType::Read);
    if err {
        return;
    }
    phys_address &= !3;

    n64.cpu.registers.gpr[rt(opcode) as usize] = se32(
        ((n64.cpu.registers.gpr[rt(opcode) as usize] as u32) & (mask as u32)
            | memory::read_data(n64, phys_address, memory::AccessSize::Word, cached) >> shift)
            as i32,
    )
}

pub(super) fn lwu(n64: &mut N64, opcode: u32) {
    let (phys_address, cached, err) = memory::translate_address(
        n64,
        n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        memory::AccessType::Read,
    );
    if err {
        return;
    }

    n64.cpu.registers.gpr[rt(opcode) as usize] =
        memory::read_data(n64, phys_address, memory::AccessSize::Word, cached) as u64
}

pub(super) fn sb(n64: &mut N64, opcode: u32) {
    let (phys_address, cached, err) = memory::translate_address(
        n64,
        n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        memory::AccessType::Write,
    );
    if err {
        return;
    }

    let shift = bshift(phys_address);

    memory::write_data(
        n64,
        phys_address & !0x3,
        (n64.cpu.registers.gpr[rt(opcode) as usize] as u32) << shift,
        0xFF << shift,
        cached,
    )
}

pub(super) fn sh(n64: &mut N64, opcode: u32) {
    let (phys_address, cached, err) = memory::translate_address(
        n64,
        n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        memory::AccessType::Write,
    );
    if err {
        return;
    }

    let shift = hshift(phys_address);

    memory::write_data(
        n64,
        phys_address & !0x3,
        (n64.cpu.registers.gpr[rt(opcode) as usize] as u32) << shift,
        0xFFFF << shift,
        cached,
    )
}

pub(super) fn swl(n64: &mut N64, opcode: u32) {
    let addr = n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16));
    let n = addr & 3;
    let shift = 8 * n;

    let mask = if n == 0 {
        u32::MAX
    } else {
        bits_below_mask(8 * (4 - n)) as u32
    };

    let (mut phys_address, cached, err) =
        memory::translate_address(n64, addr, memory::AccessType::Write);
    if err {
        return;
    }
    phys_address &= !3;

    memory::write_data(
        n64,
        phys_address,
        (n64.cpu.registers.gpr[rt(opcode) as usize] >> shift) as u32,
        mask,
        cached,
    )
}

pub(super) fn sw(n64: &mut N64, opcode: u32) {
    let (phys_address, cached, err) = memory::translate_address(
        n64,
        n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        memory::AccessType::Write,
    );
    if err {
        return;
    }

    memory::write_data(
        n64,
        phys_address,
        n64.cpu.registers.gpr[rt(opcode) as usize] as u32,
        0xFFFFFFFF,
        cached,
    )
}

pub(super) fn sdl(n64: &mut N64, opcode: u32) {
    let addr = n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16));
    let n = addr & 7;
    let shift = 8 * n;

    let mask = if n == 0 {
        u64::MAX
    } else {
        bits_below_mask(8 * (8 - n))
    };

    let (mut phys_address, cached, err) =
        memory::translate_address(n64, addr, memory::AccessType::Write);
    if err {
        return;
    }
    phys_address &= !7;

    let value = n64.cpu.registers.gpr[rt(opcode) as usize] >> shift;
    memory::write_data(
        n64,
        phys_address,
        (value >> 32) as u32,
        (mask >> 32) as u32,
        cached,
    );
    memory::write_data(n64, phys_address + 4, value as u32, mask as u32, cached)
}

pub(super) fn sdr(n64: &mut N64, opcode: u32) {
    let addr = n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16));
    let n = addr & 7;
    let shift = 8 * (7 - n);

    let mask = bits_above_mask(8 * (7 - n));

    let (mut phys_address, cached, err) =
        memory::translate_address(n64, addr, memory::AccessType::Write);
    if err {
        return;
    }
    phys_address &= !7;

    let value = n64.cpu.registers.gpr[rt(opcode) as usize] << shift;
    memory::write_data(
        n64,
        phys_address,
        (value >> 32) as u32,
        (mask >> 32) as u32,
        cached,
    );
    memory::write_data(n64, phys_address + 4, value as u32, mask as u32, cached)
}

pub(super) fn swr(n64: &mut N64, opcode: u32) {
    let addr = n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16));
    let n = addr & 3;
    let shift = 8 * (3 - n);

    let mask = bits_above_mask(8 * (3 - n)) as u32;

    let (mut phys_address, cached, err) =
        memory::translate_address(n64, addr, memory::AccessType::Write);
    if err {
        return;
    }
    phys_address &= !3;

    memory::write_data(
        n64,
        phys_address,
        (n64.cpu.registers.gpr[rt(opcode) as usize] << shift) as u32,
        mask,
        cached,
    )
}

pub(super) fn ll(n64: &mut N64, opcode: u32) {
    let (phys_address, cached, err) = memory::translate_address(
        n64,
        n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        memory::AccessType::Read,
    );
    if err {
        return;
    }

    n64.cpu.registers.gpr[rt(opcode) as usize] =
        se32(memory::read_data(n64, phys_address, memory::AccessSize::Word, cached) as i32);
    n64.cpu.registers.llbit = true;
    n64.cpu.mmu.regs[MmuRegister::LLAddr as usize] = phys_address >> 4
}

pub(super) fn lld(n64: &mut N64, opcode: u32) {
    let (phys_address, cached, err) = memory::translate_address(
        n64,
        n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        memory::AccessType::Read,
    );
    if err {
        return;
    }

    let mut w = [0; 2];
    w[0] = memory::read_data(n64, phys_address, memory::AccessSize::Dword, cached);
    w[1] = memory::read_data(n64, phys_address + 4, memory::AccessSize::Dword, cached);
    n64.cpu.registers.gpr[rt(opcode) as usize] = ((w[0] as u64) << 32) | (w[1]) as u64;

    n64.cpu.registers.llbit = true;
    n64.cpu.mmu.regs[MmuRegister::LLAddr as usize] = phys_address >> 4
}

pub(super) fn ld(n64: &mut N64, opcode: u32) {
    let (phys_address, cached, err) = memory::translate_address(
        n64,
        n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        memory::AccessType::Read,
    );
    if err {
        return;
    }

    let mut w = [0; 2];
    w[0] = memory::read_data(n64, phys_address, memory::AccessSize::Dword, cached);
    w[1] = memory::read_data(n64, phys_address + 4, memory::AccessSize::Dword, cached);
    n64.cpu.registers.gpr[rt(opcode) as usize] = ((w[0] as u64) << 32) | (w[1]) as u64
}

pub(super) fn sc(n64: &mut N64, opcode: u32) {
    if n64.cpu.registers.llbit {
        let (phys_address, cached, err) = memory::translate_address(
            n64,
            n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
            memory::AccessType::Write,
        );
        if err {
            return;
        }

        n64.cpu.registers.llbit = false;
        memory::write_data(
            n64,
            phys_address,
            n64.cpu.registers.gpr[rt(opcode) as usize] as u32,
            0xFFFFFFFF,
            cached,
        );
        n64.cpu.registers.gpr[rt(opcode) as usize] = 1
    } else {
        n64.cpu.registers.gpr[rt(opcode) as usize] = 0
    }
}

pub(super) fn scd(n64: &mut N64, opcode: u32) {
    if n64.cpu.registers.llbit {
        let (phys_address, cached, err) = memory::translate_address(
            n64,
            n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
            memory::AccessType::Write,
        );
        if err {
            return;
        }

        n64.cpu.registers.llbit = false;
        memory::write_data(
            n64,
            phys_address,
            (n64.cpu.registers.gpr[rt(opcode) as usize] >> 32) as u32,
            0xFFFFFFFF,
            cached,
        );
        memory::write_data(
            n64,
            phys_address + 4,
            n64.cpu.registers.gpr[rt(opcode) as usize] as u32,
            0xFFFFFFFF,
            cached,
        );
        n64.cpu.registers.gpr[rt(opcode) as usize] = 1
    } else {
        n64.cpu.registers.gpr[rt(opcode) as usize] = 0
    }
}

pub(super) fn sd(n64: &mut N64, opcode: u32) {
    let (phys_address, cached, err) = memory::translate_address(
        n64,
        n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        memory::AccessType::Write,
    );
    if err {
        return;
    }

    memory::write_data(
        n64,
        phys_address,
        (n64.cpu.registers.gpr[rt(opcode) as usize] >> 32) as u32,
        0xFFFFFFFF,
        cached,
    );
    memory::write_data(
        n64,
        phys_address + 4,
        n64.cpu.registers.gpr[rt(opcode) as usize] as u32,
        0xFFFFFFFF,
        cached,
    )
}
