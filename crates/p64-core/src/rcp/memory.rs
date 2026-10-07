use super::check_rcp_access;
use crate::{
    cpu, exceptions,
    memory::{self, AccessSize, AccessType},
    mmu::{self, MmuCause},
    N64,
};

const WORD_BYTES: u64 = 4;
const DOUBLEWORD_BYTES: u64 = 8;

fn translate(n64: &mut N64, opcode: u32, access: AccessType, width: u64) -> Option<(u64, bool)> {
    if !check_rcp_access(n64) {
        mmu::add_cycles(n64, 1);
        exceptions::cop_unusable_exception(n64, MmuCause::CE2 as u64);
        return None;
    }
    let address = cpu::effective_address(n64, opcode);
    if !crate::bits::is_aligned(address, width) {
        exceptions::address_error_exception(n64, address, access);
        return None;
    }
    let (physical, cached, failed) = memory::translate_address(n64, address, access);
    (!failed).then(|| (memory::reverse_endian_address(n64, physical, width), cached))
}

fn load(n64: &mut N64, opcode: u32, width: u64) {
    let Some((address, cached)) = translate(n64, opcode, AccessType::Read, width) else {
        return;
    };
    // Hardware's LWC2 samples the entire aligned 64-bit bus value at either word
    // offset. Memory transfers retain indexed values independently of MFC2's latch.
    let value = memory::read_doubleword(
        n64,
        crate::bits::align_down(address, DOUBLEWORD_BYTES),
        cached,
    );
    let rcp = n64.rcp_mut();
    rcp.reg_latch = value;
    rcp.memory_registers[cpu::rt(opcode) as usize] = value;
}

fn store(n64: &mut N64, opcode: u32, width: u64) {
    let Some((address, cached)) = translate(n64, opcode, AccessType::Write, width) else {
        return;
    };
    let value = n64.rcp().memory_registers[cpu::rt(opcode) as usize];
    if width == DOUBLEWORD_BYTES {
        memory::write_data(n64, address, (value >> u32::BITS) as u32, u32::MAX, cached);
        memory::write_data(n64, address + WORD_BYTES, value as u32, u32::MAX, cached);
    } else {
        memory::write_data(n64, address, value as u32, u32::MAX, cached);
    }
}

pub(crate) fn lwc2(n64: &mut N64, opcode: u32) {
    load(n64, opcode, AccessSize::Word as u64);
}

pub(crate) fn ldc2(n64: &mut N64, opcode: u32) {
    load(n64, opcode, AccessSize::Dword as u64);
}

pub(crate) fn swc2(n64: &mut N64, opcode: u32) {
    store(n64, opcode, AccessSize::Word as u64);
}

pub(crate) fn sdc2(n64: &mut N64, opcode: u32) {
    store(n64, opcode, AccessSize::Dword as u64);
}
