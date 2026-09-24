use super::*;

pub(super) fn lqv(n64: &mut N64, opcode: u32) {
    let mut address = n64.rsp.registers.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 4));

    let mut element = velement(opcode);
    let end = std::cmp::min(16 + element - ((address & 15) as u8), 16);
    while element < end {
        modify_vpr_byte(
            &mut n64.rsp.registers.vpr[rt(opcode) as usize],
            n64.rsp.memory.mem[(address & ADDR_MASK) as usize],
            element,
        );
        address += 1;
        element += 1;
    }
}

pub(super) fn read_memory(n64: &N64, address: u32) -> u8 {
    n64.rsp.memory.mem[(address & ADDR_MASK) as usize]
}

pub(super) fn lfv(n64: &mut N64, opcode: u32) {
    let mut address = calculate_address(n64, opcode);
    let index = ((address & 7) as u8).wrapping_sub(velement(opcode));
    address &= !7;
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
    const VPR_GROUP_SIZE: usize = 8;
    const ADDRESS_MASK: u32 = 15;
    const ADDRESS_ALIGN: u32 = !7;
    const VT_OFFSET_MASK: u32 = 7;

    let base_address = calculate_address(n64, opcode);
    let begin = base_address & ADDRESS_ALIGN;
    let initial_address = begin + (((velement(opcode)) as u32 + (base_address & 8)) & ADDRESS_MASK);
    let vt_base = rt(opcode) & !7;
    let initial_vt_offset = (velement(opcode)) as u32 >> 1;

    (0..VPR_GROUP_SIZE).fold(
        (initial_address, initial_vt_offset),
        |(mut addr, vt_off), i| {
            // First byte
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

            // Second byte
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

            (addr, (vt_off + 1) & VT_OFFSET_MASK)
        },
    );
}

pub(super) fn sbv(n64: &mut N64, opcode: u32) {
    let address = n64.rsp.registers.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 0));

    n64.rsp.memory.mem[(address & ADDR_MASK) as usize] =
        get_vpr_byte(n64.rsp.registers.vpr[rt(opcode) as usize], velement(opcode))
}

pub(super) fn calculate_address(n64: &mut N64, opcode: u32) -> u32 {
    n64.rsp.registers.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 4))
}

pub(super) fn write_memory(n64: &mut N64, address: u32, value: u8) {
    n64.rsp.memory.mem[(address & ADDR_MASK) as usize] = value
}

pub(super) fn srv(n64: &mut N64, opcode: u32) {
    const SRV_ADDR_ALIGN_MASK: u32 = !15;
    const ADDR_OFFSET_MASK: u32 = 15;
    const VPR_SIZE: u8 = 16;

    let address = calculate_address(n64, opcode);
    let aligned_addr = address & SRV_ADDR_ALIGN_MASK;
    let offset = (address & ADDR_OFFSET_MASK) as u8;
    let start_element = velement(opcode);
    let end_element = start_element + offset;
    let base = VPR_SIZE - offset;

    // Pre-calculate all values
    let values: Vec<u8> = (start_element..end_element)
        .map(|element| get_vpr_byte_value(n64, opcode, element + base))
        .collect();

    // Perform memory writes
    for (i, value) in values.iter().enumerate() {
        write_memory(n64, aligned_addr + i as u32, *value);
    }
}

pub(super) fn shv(n64: &mut N64, opcode: u32) {
    let base_address = calculate_address(n64, opcode);
    let start_element = velement(opcode);
    let initial_index = (base_address & 7) as u8;
    let aligned_address = base_address & ADDR_ALIGN_MASK;

    // Calculate all byte values first
    let byte_values: Vec<u8> = (0..VECTOR_SIZE)
        .map(|offset| {
            let byte_val = start_element + (offset * 2) as u8;
            (get_vpr_byte_value(n64, opcode, byte_val) << BYTE_SHIFT_LEFT)
                | (get_vpr_byte_value(n64, opcode, byte_val + 1) >> BYTE_SHIFT_RIGHT)
        })
        .collect();

    // Perform memory writes with calculated values
    byte_values.iter().enumerate().for_each(|(offset, &value)| {
        let current_index = ((initial_index as usize + offset * 2) & BASE_MASK as usize) as u32;
        write_memory(n64, aligned_address + current_index, value);
    });
}

pub(super) fn sfv(n64: &mut N64, opcode: u32) {
    let address = calculate_address(n64, opcode);
    let base = address & 7;
    let aligned_address = address & ADDR_ALIGN_MASK;
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
                let offset = ((base + i) & BASE_MASK) as u32;
                write_memory(n64, aligned_address + offset, 0);
            });
        }
    }
}

pub(super) fn swv(n64: &mut N64, opcode: u32) {
    let base_address = n64.rsp.registers.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 4));

    let start_element = velement(opcode);
    let initial_base = base_address & 7;
    let aligned_address = base_address & ADDR_ALIGN_MASK;
    let vpr_register = rt(opcode) as usize;

    (0..ELEMENT_COUNT).for_each(|offset| {
        let current_element = start_element + offset;
        let current_base = initial_base.wrapping_add(offset as u32);

        let mem_addr = ((aligned_address + (current_base & BASE_MASK)) & ADDR_MASK) as usize;
        n64.rsp.memory.mem[mem_addr] =
            get_vpr_byte(n64.rsp.registers.vpr[vpr_register], current_element);
    });
}

pub(super) fn stv(n64: &mut N64, opcode: u32) {
    let base_address = n64.rsp.registers.gpr[rs(opcode) as usize]
        .wrapping_add(sign_extend_7bit_offset(voffset(opcode), 4));

    let start_reg = rt(opcode) & !7;
    let start_element = VPR_ELEMENTS - ((velement(opcode) & !1) as usize);
    let base_offset = (base_address & 7).wrapping_sub((velement(opcode) & !1) as u32);
    let aligned_address = base_address & !7;

    let mut write_vpr_byte = |offset: usize, element: usize, base: u32| {
        let mem_addr = ((aligned_address + (base & 15)) & ADDR_MASK) as usize;
        n64.rsp.memory.mem[mem_addr] = get_vpr_byte(n64.rsp.registers.vpr[offset], element as u8);
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
