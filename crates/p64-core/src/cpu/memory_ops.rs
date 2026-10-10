use super::{se16, se32};
use crate::{memory, mmu::MmuRegister, N64};
use address::{MemoryOperand, PhysicalAccess};
use merge::{MergeLane, MergeSide};

const RDRAM_END: u64 = 0x0080_0000;
const SP_MEMORY_START: u64 = 0x0400_0000;
const SP_MEMORY_END: u64 = 0x0404_0000;
const LL_ADDRESS_SHIFT: u64 = 4;

mod address;
mod merge;

pub(super) use address::check_alignment;

#[inline]
pub(crate) fn effective_address(n64: &N64, opcode: u32) -> u64 {
    MemoryOperand::decode(n64, opcode).address
}

pub(super) fn translate_aligned_address(
    n64: &mut N64,
    address: u64,
    access: memory::AccessType,
    alignment: u64,
) -> (u64, bool, bool) {
    let operand = MemoryOperand {
        register: 0,
        address,
    };

    match operand.aligned(n64, access, alignment) {
        Some(physical) => (physical.address, physical.cached, false),
        None => (0, false, true),
    }
}

#[inline(always)]
fn load<const WIDTH: u64>(
    n64: &mut N64,
    opcode: u32,
    extend: impl FnOnce(u64) -> u64,
    linked: bool,
) {
    let operand = MemoryOperand::decode(n64, opcode);
    let Some(physical) = operand.aligned(n64, memory::AccessType::Read, WIDTH) else {
        return;
    };

    let value = extend(physical.read::<WIDTH>(n64));
    n64.cpu_mut().set_gpr(operand.register, value);

    if linked {
        n64.cpu_mut().registers.llbit = true;
        n64.set_cp0_register(MmuRegister::LLAddr, physical.address >> LL_ADDRESS_SHIFT);
    }
}

#[inline(always)]
fn store<const WIDTH: u64>(n64: &mut N64, opcode: u32) {
    let operand = MemoryOperand::decode(n64, opcode);
    let Some(physical) = operand.aligned(n64, memory::AccessType::Write, WIDTH) else {
        return;
    };

    physical.write::<WIDTH>(n64, n64.cpu().gpr(operand.register));
}

fn load_merge<const WIDTH: u64>(n64: &mut N64, opcode: u32, side: MergeSide) {
    let operand = MemoryOperand::decode(n64, opcode);
    let Some((physical, offset)) = operand.merged(n64, memory::AccessType::Read, WIDTH) else {
        return;
    };

    let lane = MergeLane::new::<WIDTH>(offset, side);
    let previous = n64.cpu().gpr(operand.register);
    let value = physical.read::<WIDTH>(n64);
    let previous = if WIDTH == 8 {
        n64.cpu().gpr(operand.register)
    } else {
        previous
    };
    let merged = match (WIDTH, side) {
        (4, MergeSide::Left) => se32(lane.load(previous as u32 as u64, value) as i32),
        (4, MergeSide::Right) if offset == 3 => se32(value as i32),
        _ => lane.load(previous, value),
    };

    n64.cpu_mut().set_gpr(operand.register, merged);
}

fn store_merge<const WIDTH: u64>(n64: &mut N64, opcode: u32, side: MergeSide) {
    let operand = MemoryOperand::decode(n64, opcode);
    let Some((physical, offset)) = operand.merged(n64, memory::AccessType::Write, WIDTH) else {
        return;
    };

    let lane = MergeLane::new::<WIDTH>(offset, side);
    let value = lane.store(n64.cpu().gpr(operand.register));

    if WIDTH == 8 {
        physical.write_doubleword(n64, value, lane.store_mask);
    } else {
        physical.write_word(n64, value as u32, lane.store_mask as u32);
    }
}

fn store_conditional<const WIDTH: u64>(n64: &mut N64, opcode: u32) {
    let operand = MemoryOperand::decode(n64, opcode);
    // Translation still raises faults when the reservation is absent.
    let Some(physical) = operand.aligned(n64, memory::AccessType::Write, WIDTH) else {
        return;
    };

    let success = n64.cpu().registers.llbit;

    if success {
        let value = n64.cpu().gpr(operand.register);

        if WIDTH == 8 {
            physical.write_word(n64, (value >> 32) as u32, u32::MAX);
            PhysicalAccess {
                address: physical.address + 4,
                ..physical
            }
            .write_word(n64, n64.cpu().gpr(operand.register) as u32, u32::MAX);
        } else {
            physical.write::<WIDTH>(n64, value);
        }
    }
    // VR4300 SC does not clear LLBit or compare LLAddr.
    n64.cpu_mut().set_gpr(operand.register, u64::from(success));
}

pub(super) fn lb(n64: &mut N64, opcode: u32) {
    load::<1>(n64, opcode, |v| v as u8 as i8 as i64 as u64, false);
}

pub(super) fn lh(n64: &mut N64, opcode: u32) {
    load::<2>(n64, opcode, |v| se16(v as i16), false);
}

pub(super) fn lw(n64: &mut N64, opcode: u32) {
    load::<4>(n64, opcode, |v| se32(v as i32), false);
}

pub(super) fn lbu(n64: &mut N64, opcode: u32) {
    load::<1>(n64, opcode, |v| v as u8 as u64, false);
}

pub(super) fn lhu(n64: &mut N64, opcode: u32) {
    load::<2>(n64, opcode, |v| v as u16 as u64, false);
}

pub(super) fn lwu(n64: &mut N64, opcode: u32) {
    load::<4>(n64, opcode, |v| v, false);
}

pub(super) fn ld(n64: &mut N64, opcode: u32) {
    load::<8>(n64, opcode, |v| v, false);
}

pub(super) fn ll(n64: &mut N64, opcode: u32) {
    load::<4>(n64, opcode, |v| se32(v as i32), true);
}

pub(super) fn lld(n64: &mut N64, opcode: u32) {
    load::<8>(n64, opcode, |v| v, true);
}

pub(super) fn lwl(n64: &mut N64, opcode: u32) {
    load_merge::<4>(n64, opcode, MergeSide::Left);
}

pub(super) fn lwr(n64: &mut N64, opcode: u32) {
    load_merge::<4>(n64, opcode, MergeSide::Right);
}

pub(super) fn ldl(n64: &mut N64, opcode: u32) {
    load_merge::<8>(n64, opcode, MergeSide::Left);
}

pub(super) fn ldr(n64: &mut N64, opcode: u32) {
    load_merge::<8>(n64, opcode, MergeSide::Right);
}

pub(super) fn sb(n64: &mut N64, opcode: u32) {
    store::<1>(n64, opcode);
}

pub(super) fn sh(n64: &mut N64, opcode: u32) {
    store::<2>(n64, opcode);
}

pub(super) fn sw(n64: &mut N64, opcode: u32) {
    store::<4>(n64, opcode);
}

pub(super) fn sc(n64: &mut N64, opcode: u32) {
    store_conditional::<4>(n64, opcode);
}

pub(super) fn scd(n64: &mut N64, opcode: u32) {
    store_conditional::<8>(n64, opcode);
}

pub(super) fn swl(n64: &mut N64, opcode: u32) {
    store_merge::<4>(n64, opcode, MergeSide::Left);
}

pub(super) fn swr(n64: &mut N64, opcode: u32) {
    store_merge::<4>(n64, opcode, MergeSide::Right);
}

pub(super) fn sdl(n64: &mut N64, opcode: u32) {
    store_merge::<8>(n64, opcode, MergeSide::Left);
}

pub(super) fn sdr(n64: &mut N64, opcode: u32) {
    store_merge::<8>(n64, opcode, MergeSide::Right);
}

pub(super) fn sd(n64: &mut N64, opcode: u32) {
    let operand = MemoryOperand::decode(n64, opcode);
    let Some(physical) = operand.aligned(n64, memory::AccessType::Write, 8) else {
        return;
    };

    let value = n64.cpu().gpr(operand.register);

    if !physical.cached && physical.address < RDRAM_END && crate::mips::repeat_enabled(n64) {
        crate::rdram::write_mem_double_repeat(n64, physical.address, value);
        return;
    }

    physical.write_word(n64, (value >> 32) as u32, u32::MAX);
    // The SP memory bus accepts only the first word of a doubleword store.
    if !physical.cached && (SP_MEMORY_START..SP_MEMORY_END).contains(&physical.address) {
        return;
    }

    PhysicalAccess {
        address: physical.address + 4,
        ..physical
    }
    .write_word(n64, n64.cpu().gpr(operand.register) as u32, u32::MAX);
}
