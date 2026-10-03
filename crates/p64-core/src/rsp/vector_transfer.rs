use super::{
    get_vpr_byte, get_vpr_element, modify_vpr_byte, modify_vpr_element, rt, velement, Rsp,
    ADDR_MASK, BASE_MASK, BYTE_SHIFT_LEFT, BYTE_SHIFT_RIGHT, ELEMENT_COUNT, ELEMENT_MASK,
    ELEMENT_PATTERNS, GROUP_ALIGNMENT, VECTOR_ALIGNMENT, VECTOR_SIZE, VPR_ELEMENTS, VPR_SHIFT,
    VPR_SIZE,
};
use crate::N64;

#[inline]
fn copy_vector_bytes(rsp: &mut Rsp, opcode: u32, address: u32, start: u8, end: u8) {
    for (offset, element) in (start..end).enumerate() {
        let address = (address + offset as u32) & ADDR_MASK;
        let value = rsp.memory.mem[address as usize];
        modify_vpr_byte(&mut rsp.registers.vpr[rt(opcode) as usize], value, element);
    }
}

#[inline]
fn load_bytes(rsp: &mut Rsp, opcode: u32, scale: u32, count: u8) {
    let address = rsp.registers.vector_address(opcode, scale);
    let start = velement(opcode);
    copy_vector_bytes(rsp, opcode, address, start, (start + count).min(VPR_SIZE));
}

#[inline]
fn load_packed(rsp: &mut Rsp, opcode: u32, scale: u32, stride: u8, shift: u32) {
    let address = rsp.registers.vector_address(opcode, scale);
    let index = ((address & 7) as u8).wrapping_sub(velement(opcode));
    let aligned = crate::bits::align_down(address, GROUP_ALIGNMENT);

    for offset in 0..VECTOR_SIZE as u8 {
        let element = index.wrapping_add(offset * stride) & ELEMENT_MASK as u8;
        let address = (aligned + u32::from(element)) & ADDR_MASK;
        let value = u16::from(rsp.memory.mem[address as usize]) << shift;
        modify_vpr_element(&mut rsp.registers.vpr[rt(opcode) as usize], value, offset);
    }
}

pub(super) fn lbv(n64: &mut N64, opcode: u32) {
    load_bytes(n64.rsp_mut(), opcode, 0, 1);
}

pub(super) fn lsv(n64: &mut N64, opcode: u32) {
    load_bytes(n64.rsp_mut(), opcode, 1, 2);
}

pub(super) fn llv(n64: &mut N64, opcode: u32) {
    load_bytes(n64.rsp_mut(), opcode, 2, 4);
}

pub(super) fn ldv(n64: &mut N64, opcode: u32) {
    load_bytes(n64.rsp_mut(), opcode, 3, 8);
}

pub(super) fn lrv(n64: &mut N64, opcode: u32) {
    let rsp = n64.rsp_mut();
    let address = rsp.registers.vector_address(opcode, 4);
    let start =
        VPR_SIZE.wrapping_sub(((address & ELEMENT_MASK) as u8).wrapping_sub(velement(opcode)));
    let aligned = crate::bits::align_down(address, VECTOR_ALIGNMENT);
    copy_vector_bytes(rsp, opcode, aligned, start, VPR_SIZE);
}

pub(super) fn lpv(n64: &mut N64, opcode: u32) {
    load_packed(n64.rsp_mut(), opcode, 3, 1, 8);
}

pub(super) fn luv(n64: &mut N64, opcode: u32) {
    load_packed(n64.rsp_mut(), opcode, 3, 1, 7);
}

pub(super) fn lhv(n64: &mut N64, opcode: u32) {
    load_packed(n64.rsp_mut(), opcode, 4, 2, 7);
}

pub(super) fn lqv(n64: &mut N64, opcode: u32) {
    let rsp = n64.rsp_mut();
    let address = rsp.registers.vector_address(opcode, 4);
    let start = velement(opcode);
    let end = (VPR_SIZE + start - (address & ELEMENT_MASK) as u8).min(VPR_SIZE);
    copy_vector_bytes(rsp, opcode, address, start, end);
}

fn read_memory(n64: &N64, address: u32) -> u8 {
    n64.rsp().memory.mem[(address & ADDR_MASK) as usize]
}

pub(super) fn lfv(n64: &mut N64, opcode: u32) {
    let mut address = calculate_address(n64, opcode);
    let index = ((address & 7) as u8).wrapping_sub(velement(opcode));
    address = crate::bits::align_down(address, GROUP_ALIGNMENT);
    let start = velement(opcode);
    let end = std::cmp::min(start + 8, 16);
    let mut tmp: u128 = 0;
    let mut offset: u8 = 0;

    while offset < 4 {
        modify_vpr_element(
            &mut tmp,
            (read_memory(
                n64,
                address.wrapping_add(((index.wrapping_add(offset * 4)) & 15) as u32),
            ) as u16)
                << 7,
            offset,
        );
        modify_vpr_element(
            &mut tmp,
            (read_memory(
                n64,
                address
                    .wrapping_add(((index.wrapping_add(offset * 4).wrapping_add(8)) & 15) as u32),
            ) as u16)
                << 7,
            offset + 4,
        );
        offset += 1;
    }
    offset = start;

    while offset < end {
        let value = get_vpr_byte(tmp, offset);
        modify_vpr_byte(
            &mut n64.rsp.registers.vpr[rt(opcode) as usize],
            value,
            offset,
        );
        offset += 1;
    }
}

pub(super) fn lwv(_device: &mut N64, _opcode: u32) {}

pub(super) fn ltv(n64: &mut N64, opcode: u32) {
    let base_address = calculate_address(n64, opcode);
    let begin = crate::bits::align_down(base_address, GROUP_ALIGNMENT);
    let initial_address = begin + (((velement(opcode)) as u32 + (base_address & 8)) & BASE_MASK);
    let vt_base = crate::bits::align_down(rt(opcode), GROUP_ALIGNMENT);
    let initial_vt_offset = (velement(opcode)) as u32 >> 1;

    (0..VECTOR_SIZE).fold(
        (initial_address, initial_vt_offset),
        |(mut addr, vt_off), i| {
            let value1 = read_memory(n64, addr);
            modify_vpr_byte(
                &mut n64.rsp.registers.vpr[(vt_base + vt_off) as usize],
                value1,
                (i * 2) as u8,
            );
            addr = if (addr + 1) == begin + 16 {
                begin
            } else {
                addr + 1
            };

            let value2 = read_memory(n64, addr);
            modify_vpr_byte(
                &mut n64.rsp.registers.vpr[(vt_base + vt_off) as usize],
                value2,
                (i * 2 + 1) as u8,
            );
            addr = if (addr + 1) == begin + 16 {
                begin
            } else {
                addr + 1
            };

            (addr, (vt_off + 1) & (VECTOR_SIZE as u32 - 1))
        },
    );
}

pub(super) fn sbv(n64: &mut N64, opcode: u32) {
    let rsp = n64.rsp_mut();
    let address = rsp.registers.vector_address(opcode, 0);

    rsp.memory.mem[(address & ADDR_MASK) as usize] =
        get_vpr_byte(rsp.registers.vpr[rt(opcode) as usize], velement(opcode))
}

#[inline]
fn store_vector(
    rsp: &mut Rsp,
    opcode: u32,
    address: u32,
    count: u8,
    byte: impl Fn(u128, u8) -> u8,
) {
    let start = velement(opcode);
    let register = rsp.registers.vpr[rt(opcode) as usize];

    for offset in 0..count {
        let address = address.wrapping_add(u32::from(offset)) & ADDR_MASK;
        rsp.memory.mem[address as usize] = byte(register, start.wrapping_add(offset));
    }
}

#[inline]
fn store_bytes(rsp: &mut Rsp, opcode: u32, scale: u32, count: u8) {
    let address = rsp.registers.vector_address(opcode, scale);
    store_vector(rsp, opcode, address, count, get_vpr_byte);
}

#[inline]
fn packed_byte(register: u128, element: u8, unsigned: bool) -> u8 {
    let first_half = (element & ELEMENT_MASK as u8) < VECTOR_SIZE as u8;

    if first_half != unsigned {
        get_vpr_byte(register, (element & 7) << 1)
    } else {
        (get_vpr_element(register, element) >> VPR_SHIFT) as u8
    }
}

pub(super) fn ssv(n64: &mut N64, opcode: u32) {
    store_bytes(n64.rsp_mut(), opcode, 1, 2);
}

pub(super) fn slv(n64: &mut N64, opcode: u32) {
    store_bytes(n64.rsp_mut(), opcode, 2, 4);
}

pub(super) fn sdv(n64: &mut N64, opcode: u32) {
    store_bytes(n64.rsp_mut(), opcode, 3, 8);
}

pub(super) fn sqv(n64: &mut N64, opcode: u32) {
    let rsp = n64.rsp_mut();
    let address = rsp.registers.vector_address(opcode, 4);
    let count = VPR_SIZE - (address & ELEMENT_MASK) as u8;
    store_vector(rsp, opcode, address, count, get_vpr_byte);
}

pub(super) fn spv(n64: &mut N64, opcode: u32) {
    let rsp = n64.rsp_mut();
    let address = rsp.registers.vector_address(opcode, 3);
    store_vector(
        rsp,
        opcode,
        address,
        VECTOR_SIZE as u8,
        |register, element| packed_byte(register, element, false),
    );
}

pub(super) fn suv(n64: &mut N64, opcode: u32) {
    let rsp = n64.rsp_mut();
    let address = rsp.registers.vector_address(opcode, 3);
    store_vector(
        rsp,
        opcode,
        address,
        VECTOR_SIZE as u8,
        |register, element| packed_byte(register, element, true),
    );
}

fn calculate_address(n64: &N64, opcode: u32) -> u32 {
    let rsp = n64.rsp();

    rsp.registers.vector_address(opcode, 4)
}

fn write_memory(n64: &mut N64, address: u32, value: u8) {
    let rsp = n64.rsp_mut();

    rsp.memory.mem[(address & ADDR_MASK) as usize] = value
}

pub(super) fn srv(n64: &mut N64, opcode: u32) {
    let rsp = n64.rsp_mut();
    let address = rsp.registers.vector_address(opcode, 4);
    let aligned = crate::bits::align_down(address, VECTOR_ALIGNMENT);
    let count = (address & BASE_MASK) as u8;
    let start = velement(opcode);
    let base = VPR_SIZE - count;
    let register = rsp.registers.vpr[rt(opcode) as usize];

    for offset in 0..count {
        rsp.memory.mem[((aligned + u32::from(offset)) & ADDR_MASK) as usize] =
            get_vpr_byte(register, start + offset + base);
    }
}

pub(super) fn shv(n64: &mut N64, opcode: u32) {
    let rsp = n64.rsp_mut();
    let address = rsp.registers.vector_address(opcode, 4);
    let start = velement(opcode);
    let index = address & 7;
    let aligned = crate::bits::align_down(address, GROUP_ALIGNMENT);
    let register = rsp.registers.vpr[rt(opcode) as usize];

    for offset in 0..VECTOR_SIZE {
        let element = start + (offset * 2) as u8;
        let value = (get_vpr_byte(register, element) << BYTE_SHIFT_LEFT)
            | (get_vpr_byte(register, element + 1) >> BYTE_SHIFT_RIGHT);
        let lane = (index + offset as u32 * 2) & BASE_MASK;
        rsp.memory.mem[((aligned + lane) & ADDR_MASK) as usize] = value;
    }
}

pub(super) fn sfv(n64: &mut N64, opcode: u32) {
    let address = calculate_address(n64, opcode);
    let base = address & 7;
    let aligned_address = crate::bits::align_down(address, GROUP_ALIGNMENT);
    let element = velement(opcode);
    let vpr_register = rt(opcode) as usize;

    match ELEMENT_PATTERNS.iter().find(|&&(e, _)| e == element) {
        Some((_, pattern)) => {
            pattern.0.iter().enumerate().for_each(|(i, &elem)| {
                let offset = ((base + (i * 4) as u32) & BASE_MASK) as usize;
                let vpr_value = get_vpr_element(n64.rsp.registers.vpr[vpr_register], elem as u8);
                let byte_value = (vpr_value >> VPR_SHIFT) as u8;
                write_memory(n64, aligned_address + offset as u32, byte_value);
            });
        }
        None => {
            (0..16).step_by(4).for_each(|i| {
                let offset = (base + i) & BASE_MASK;
                write_memory(n64, aligned_address + offset, 0);
            });
        }
    }
}

pub(super) fn swv(n64: &mut N64, opcode: u32) {
    let rsp = n64.rsp_mut();
    let base_address = rsp.registers.vector_address(opcode, 4);

    let start_element = velement(opcode);
    let initial_base = base_address & 7;
    let aligned_address = crate::bits::align_down(base_address, GROUP_ALIGNMENT);
    let vpr_register = rt(opcode) as usize;

    (0..ELEMENT_COUNT).for_each(|offset| {
        let current_element = start_element + offset;
        let current_base = initial_base.wrapping_add(offset as u32);

        let mem_addr = ((aligned_address + (current_base & BASE_MASK)) & ADDR_MASK) as usize;
        rsp.memory.mem[mem_addr] = get_vpr_byte(rsp.registers.vpr[vpr_register], current_element);
    });
}

pub(super) fn stv(n64: &mut N64, opcode: u32) {
    let rsp = n64.rsp_mut();
    let base_address = rsp.registers.vector_address(opcode, 4);

    let start_reg = crate::bits::align_down(rt(opcode), GROUP_ALIGNMENT);
    let start_element = VPR_ELEMENTS - ((crate::bits::align_down(velement(opcode), 2)) as usize);
    let base_offset =
        (base_address & 7).wrapping_sub((crate::bits::align_down(velement(opcode), 2)) as u32);
    let aligned_address = crate::bits::align_down(base_address, GROUP_ALIGNMENT);

    let mut write_vpr_byte = |offset: usize, element: usize, base: u32| {
        let mem_addr = ((aligned_address + (base & 15)) & ADDR_MASK) as usize;
        rsp.memory.mem[mem_addr] = get_vpr_byte(rsp.registers.vpr[offset], element as u8);
    };

    (0..VECTOR_SIZE).for_each(|i| {
        let reg_idx = start_reg + i as u32;
        let current_base = base_offset.wrapping_add(2 * i as u32);
        let current_element = start_element + 2 * i;

        write_vpr_byte(reg_idx as usize, current_element, current_base);
        write_vpr_byte(
            reg_idx as usize,
            current_element + 1,
            current_base.wrapping_add(1),
        );
    });
}
