use super::*;

pub(super) fn reference(op: u32, gpr: &mut [u64; 32]) {
    let rs = ((op >> 21) & 31) as usize;
    let rt = ((op >> 16) & 31) as usize;
    let rd = ((op >> 11) & 31) as usize;
    let sa = (op >> 6) & 31;
    let x = if rs == 0 { 0 } else { gpr[rs] };
    let y = if rt == 0 { 0 } else { gpr[rt] };

    let word = |v: u64| v as u32 as i32 as i64 as u64;
    let immediate = op as i16 as i64 as u64;
    let value = match op >> 26 {
        0 => match op & 63 {
            0 => word((y as u32 as u64) << sa),
            2 => word((y as u32 >> sa) as u64),
            3 => word(((y as i64) >> sa) as u64),
            4 => word((y as u32 as u64) << (x & 31)),
            6 => word((y as u32 >> (x & 31)) as u64),
            7 => word(((y as i64) >> (x & 31)) as u64),
            20 => y << (x & 63),
            22 => y >> (x & 63),
            23 => ((y as i64) >> (x & 63)) as u64,
            33 => word(x.wrapping_add(y)),
            35 => word(x.wrapping_sub(y)),
            36 => x & y,
            37 => x | y,
            38 => x ^ y,
            39 => !(x | y),
            42 => ((x as i64) < (y as i64)) as u64,
            43 => (x < y) as u64,
            45 => x.wrapping_add(y),
            47 => x.wrapping_sub(y),
            56 => y << sa,
            58 => y >> sa,
            59 => ((y as i64) >> sa) as u64,
            60 => y << (sa + 32),
            62 => y >> (sa + 32),
            63 => ((y as i64) >> (sa + 32)) as u64,
            _ => panic!(),
        },
        9 => word(x.wrapping_add(immediate)),
        10 => ((x as i64) < (immediate as i64)) as u64,
        11 => (x < immediate) as u64,
        12 => x & (op & 65535) as u64,
        13 => x | (op & 65535) as u64,
        14 => x ^ (op & 65535) as u64,
        15 => word(((op & 65535) << 16) as u64),
        25 => x.wrapping_add(immediate),
        _ => panic!(),
    };
    let destination = if op >> 26 == 0 { rd } else { rt };

    if destination != 0 {
        gpr[destination] = value;
    }
}

pub(super) fn memory_oracle(
    opcodes: &[u32],
    gpr: &mut [u64; 32],
    lines: &mut [crate::cache::DCache; 512],
    start_pc: u64,
    mut pending: u8,
    mut data: u64,
) -> (usize, u64, u8, u64, NativeMemoryExit) {
    let mut retired = 0;
    let mut cycles = 0;
    let mut exit = NativeMemoryExit::Complete;

    for (index, opcode) in opcodes.iter().copied().enumerate() {
        let rs = ((opcode >> 21) & 31) as usize;
        let rt = ((opcode >> 16) & 31) as usize;
        let mut address = 0;
        let mut line_index = 0;

        if supported_memory_opcode(opcode) {
            let base = if rs == 0 { 0 } else { gpr[rs] };

            address = base.wrapping_add(opcode as i16 as i64 as u64);
            if !(0xFFFF_FFFF_8000_0000..=0xFFFF_FFFF_9FFF_FFFF).contains(&address)
                || address
                    & (if matches!(opcode >> 26, 55 | 63) {
                        7
                    } else {
                        3
                    })
                    != 0
                || address & 0x1FFF_FFFF >= 0x0080_0000
            {
                exit = NativeMemoryExit::Address;
                break;
            }

            line_index = ((address >> 4) & 511) as usize;
            let line = &lines[line_index];

            if !line.valid || line.tag & 0x1FFF_FFFC != (address & 0x1FFF_F000) as u32 {
                exit = NativeMemoryExit::CacheMiss;
                break;
            }
        }

        if pending != 0 && (rs == pending as usize || rt == pending as usize) {
            cycles += 1;
        }
        pending = 0;
        if supported_memory_opcode(opcode) {
            let word = ((address >> 2) & 3) as usize;

            match opcode >> 26 {
                35 | 55 => {
                    let value = if opcode >> 26 == 55 {
                        ((lines[line_index].words[word] as u64) << 32)
                            | lines[line_index].words[word + 1] as u64
                    } else {
                        lines[line_index].words[word] as i32 as i64 as u64
                    };

                    if rt != 0 {
                        gpr[rt] = value;
                    }
                    pending = rt as u8;
                }
                43 | 63 => {
                    let value = if rt == 0 { 0 } else { gpr[rt] };

                    if opcode >> 26 == 63 {
                        lines[line_index].words[word] = (value >> 32) as u32;
                        lines[line_index].words[word + 1] = value as u32;
                    } else {
                        lines[line_index].words[word] = value as u32;
                    }
                    lines[line_index].dirty = true;
                    lines[line_index].store_dirty = true;
                }
                _ => unreachable!(),
            }
            data = address;
        } else {
            reference(opcode, gpr);
            data = start_pc.wrapping_add(index as u64 * 4);
        }
        retired += 1;
    }

    (retired, cycles, pending, data, exit)
}

pub(super) fn assert_cache_equal(
    actual: &[crate::cache::DCache; 512],
    expected: &[crate::cache::DCache; 512],
) {
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            (
                actual.valid,
                actual.dirty,
                actual.store_dirty,
                actual.tag,
                actual.index,
                actual.words
            ),
            (
                expected.valid,
                expected.dirty,
                expected.store_dirty,
                expected.tag,
                expected.index,
                expected.words
            ),
            "cache line {index}"
        );
    }
}

pub(super) fn next_random(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;

    *seed
}
