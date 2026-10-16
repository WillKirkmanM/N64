use super::cpu_snapshot;
use crate::{rsp::start_rsp, N64};

const TRANSFER_SHIFTS: [u32; 12] = [0, 1, 2, 3, 4, 4, 3, 3, 4, 4, 4, 4];

type Registers = [[u8; 16]; 32];

fn halfword(register: &[u8; 16], lane: usize) -> u16 {
    u16::from_be_bytes([register[(lane % 8) * 2], register[(lane % 8) * 2 + 1]])
}

fn load_oracle(
    function: usize,
    address: usize,
    element: usize,
    vector: usize,
    memory: &[u8; 4096],
    registers: &mut Registers,
) {
    let read = |offset: usize| memory[offset % 4096];
    match function {
        0..=4 => {
            let maximum = if function < 4 {
                1 << function
            } else {
                16 - address % 16
            };
            for offset in 0..maximum.min(16 - element) {
                registers[vector][element + offset] = read(address + offset);
            }
        }
        5 => {
            let length = (address % 16).saturating_sub(element);
            for offset in 0..length {
                registers[vector][16 - length + offset] = read(address / 16 * 16 + offset);
            }
        }
        6..=8 => {
            let stride = if function == 8 { 2 } else { 1 };
            let scale = if function == 6 { 256 } else { 128 };
            for lane in 0..8 {
                let position = (address % 8 + 16 + lane * stride - element) % 16;
                let word = u16::from(read(address / 8 * 8 + position)) * scale;
                registers[vector][lane * 2..lane * 2 + 2].copy_from_slice(&word.to_be_bytes());
            }
        }
        9 => {
            let mut temporary = [0u8; 16];
            for lane in 0..8 {
                let position = (address % 8 + 32 + (lane % 4) * 4 + (lane / 4) * 8 - element) % 16;
                let word = u16::from(read(address / 8 * 8 + position)) * 128;
                temporary[lane * 2..lane * 2 + 2].copy_from_slice(&word.to_be_bytes());
            }
            let end = (element + 8).min(16);
            registers[vector][element..end].copy_from_slice(&temporary[element..end]);
        }
        10 => {}
        11 => {
            let group = vector / 8 * 8;
            for (register, target) in registers[group..group + 8].iter_mut().enumerate() {
                let lane = (register + 8 - element / 2) % 8;
                let position = (element + address % 16 / 8 * 8 + lane * 2) % 16;
                target[lane * 2] = read(address / 8 * 8 + position);
                target[lane * 2 + 1] = read(address / 8 * 8 + (position + 1) % 16);
            }
        }
        _ => unreachable!(),
    }
}

fn store_oracle(
    function: usize,
    address: usize,
    element: usize,
    vector: usize,
    registers: &Registers,
    memory: &mut [u8; 4096],
) {
    let source = &registers[vector];
    match function {
        0..=4 => {
            let count = if function < 4 {
                1 << function
            } else {
                16 - address % 16
            };
            for offset in 0..count {
                memory[(address + offset) % 4096] = source[(element + offset) % 16];
            }
        }
        5 => {
            let count = address % 16;
            for offset in 0..count {
                memory[(address / 16 * 16 + offset) % 4096] =
                    source[(element + 16 - count + offset) % 16];
            }
        }
        6 | 7 => {
            for offset in 0..8 {
                let position = (element + offset) % 16;
                let shift = if (position < 8) == (function == 6) {
                    8
                } else {
                    7
                };
                memory[(address + offset) % 4096] = (halfword(source, position) >> shift) as u8;
            }
        }
        8 => {
            for offset in 0..8 {
                let position = (element + offset * 2) % 16;
                let pair = u16::from_be_bytes([source[position], source[(position + 1) % 16]]);
                let target = address / 8 * 8 + (address % 8 + offset * 2) % 16;
                memory[target % 4096] = (pair / 128) as u8;
            }
        }
        9 => {
            let lanes = match element {
                0 | 15 => Some([0, 1, 2, 3]),
                1 => Some([6, 7, 4, 5]),
                4 => Some([1, 2, 3, 0]),
                5 => Some([7, 4, 5, 6]),
                8 => Some([4, 5, 6, 7]),
                11 => Some([3, 0, 1, 2]),
                12 => Some([5, 6, 7, 4]),
                _ => None,
            };
            for offset in 0..4 {
                let target = address / 8 * 8 + (address % 8 + offset * 4) % 16;
                memory[target % 4096] =
                    lanes.map_or(0, |lanes| (halfword(source, lanes[offset]) / 128) as u8);
            }
        }
        10 => {
            for offset in 0..16 {
                let target = address / 8 * 8 + (address % 8 + offset) % 16;
                memory[target % 4096] = source[(element + offset) % 16];
            }
        }
        11 => {
            let aligned = address / 8 * 8;
            for offset in 0..16 {
                let source_register =
                    vector / 8 * 8 + (offset / 2 + 8 - (aligned / 2) % 8 + element / 2) % 8;
                let source_element = (offset + aligned) % 16;
                let target = aligned + (address + offset) % 16;
                memory[target % 4096] = registers[source_register][source_element];
            }
        }
        _ => unreachable!(),
    }
}

#[test]
fn every_vector_transfer_matches_byte_oracles_at_alignments_elements_offsets_and_groups() {
    let mut n64 = N64::default();
    start_rsp(&mut n64);
    let cpu = cpu_snapshot(&n64);
    let initial_memory = std::array::from_fn::<_, 4096, _>(|index| (index * 37 + index / 17) as u8);
    let initial_registers: Registers = std::array::from_fn(|register| {
        std::array::from_fn(|byte| (register * 29 + byte * 13) as u8)
    });
    for (function, shift) in TRANSFER_SHIFTS.iter().enumerate() {
        for base in [0u32, 1, 7, 8, 15, 0xff8, 0xfff, u32::MAX] {
            for immediate in [0u32, 1, 63, 64, 127] {
                for element in 0..16usize {
                    for vector in [0usize, 7, 8, 31] {
                        let displacement = if immediate < 64 {
                            immediate as i64
                        } else {
                            immediate as i64 - 128
                        };
                        let address = (i64::from(base) + displacement * (1i64 << shift))
                            .rem_euclid(4096) as usize;
                        for store in [false, true] {
                            n64.rsp_mut().registers.gpr[1] = base;
                            n64.rsp_mut().memory.mem[..4096].copy_from_slice(&initial_memory);
                            n64.rsp_mut().registers.vpr =
                                initial_registers.map(u128::from_be_bytes);
                            let mut expected_memory = initial_memory;
                            let mut expected_registers = initial_registers;
                            if store {
                                store_oracle(
                                    function,
                                    address,
                                    element,
                                    vector,
                                    &initial_registers,
                                    &mut expected_memory,
                                );
                            } else {
                                load_oracle(
                                    function,
                                    address,
                                    element,
                                    vector,
                                    &initial_memory,
                                    &mut expected_registers,
                                );
                            }
                            let primary = if store { 58 } else { 50 };
                            let opcode = (primary << 26)
                                | (1 << 21)
                                | ((vector as u32) << 16)
                                | ((function as u32) << 11)
                                | ((element as u32) << 7)
                                | immediate;
                            let handler = n64.rsp().instruction_tables.decode(opcode);
                            handler(&mut n64, opcode);
                            assert_eq!(
                                n64.rsp().registers.vpr.map(u128::to_be_bytes),
                                expected_registers,
                                "opcode={opcode:08x} base={base:08x}"
                            );
                            assert_eq!(
                                &n64.rsp().memory.mem[..4096],
                                expected_memory.as_slice(),
                                "opcode={opcode:08x} base={base:08x}"
                            );
                            assert_eq!(cpu_snapshot(&n64), cpu);
                        }
                    }
                }
            }
        }
    }
}
