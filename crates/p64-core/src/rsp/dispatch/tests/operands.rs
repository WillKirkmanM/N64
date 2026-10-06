use super::cpu_snapshot;
use crate::rsp::*;
use crate::rsp_vector::{vector_from_bits, vector_to_bits};
use crate::N64;

const WORD_EDGES: [u32; 9] = [
    0,
    1,
    31,
    32,
    0x7fff,
    0x8000,
    0x7fff_ffff,
    0x8000_0000,
    u32::MAX,
];
pub(super) const ELEMENT_LANES: [[usize; 8]; 16] = [
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 1, 2, 3, 4, 5, 6, 7],
    [0, 0, 2, 2, 4, 4, 6, 6],
    [1, 1, 3, 3, 5, 5, 7, 7],
    [0, 0, 0, 0, 4, 4, 4, 4],
    [1, 1, 1, 1, 5, 5, 5, 5],
    [2, 2, 2, 2, 6, 6, 6, 6],
    [3, 3, 3, 3, 7, 7, 7, 7],
    [0; 8],
    [1; 8],
    [2; 8],
    [3; 8],
    [4; 8],
    [5; 8],
    [6; 8],
    [7; 8],
];

pub(super) fn pack(lanes: [u16; 8]) -> u128 {
    u128::from_be_bytes(std::array::from_fn(|byte| {
        lanes[byte / 2].to_be_bytes()[byte % 2]
    }))
}

fn signed_word(value: u32) -> i64 {
    if value >= 0x8000_0000 {
        i64::from(value) - (1i64 << 32)
    } else {
        i64::from(value)
    }
}

#[test]
fn scalar_alu_slots_match_wide_arithmetic_with_register_aliases() {
    let mut n64 = N64::default();
    start_rsp(&mut n64);
    let snapshot = cpu_snapshot(&n64);
    for function in [0, 2, 3, 4, 6, 7, 32, 33, 34, 35, 36, 37, 38, 39, 42, 43] {
        for destination in [1, 2, 3, 31] {
            for left in WORD_EDGES {
                for right in WORD_EDGES {
                    for shift in [0, 1, 15, 31] {
                        n64.rsp_mut().registers.gpr[1] = left;
                        n64.rsp_mut().registers.gpr[2] = right;
                        let opcode =
                            (1 << 21) | (2 << 16) | (destination << 11) | (shift << 6) | function;
                        let amount = if function < 4 { shift } else { left % 32 };
                        let expected = match function {
                            0 | 4 => (u64::from(right) * (1u64 << amount)) as u32,
                            2 | 6 => (u64::from(right) / (1u64 << amount)) as u32,
                            3 | 7 => signed_word(right).div_euclid(1i64 << amount) as u32,
                            32 | 33 => (u64::from(left) + u64::from(right)) as u32,
                            34 | 35 => (i64::from(left) - i64::from(right)) as u32,
                            36 => left & right,
                            37 => left | right,
                            38 => left ^ right,
                            39 => u32::MAX - (left | right),
                            42 => u32::from(signed_word(left) < signed_word(right)),
                            43 => u32::from(left < right),
                            _ => unreachable!(),
                        };
                        let handler = n64.rsp().instruction_tables.decode(opcode);
                        handler(&mut n64, opcode);
                        assert_eq!(
                            n64.rsp().registers.gpr[destination as usize],
                            expected,
                            "opcode={opcode:08x} left={left:08x} right={right:08x}"
                        );
                        assert_eq!(cpu_snapshot(&n64), snapshot);
                    }
                }
            }
        }
    }
}

#[test]
fn scalar_immediate_slots_sign_extend_only_the_documented_operands() {
    let mut n64 = N64::default();
    start_rsp(&mut n64);
    let snapshot = cpu_snapshot(&n64);
    for primary in 8..=15 {
        for destination in [1, 2, 31] {
            for source in WORD_EDGES {
                for immediate in [0u32, 1, 0x7fff, 0x8000, 0xffff] {
                    n64.rsp_mut().registers.gpr[1] = source;
                    let signed = if immediate >= 0x8000 {
                        i64::from(immediate) - 0x1_0000
                    } else {
                        i64::from(immediate)
                    };
                    let expected = match primary {
                        8 | 9 => (i64::from(source) + signed) as u32,
                        10 => u32::from(signed_word(source) < signed),
                        11 => u32::from(source < signed as u32),
                        12 => source & immediate,
                        13 => source | immediate,
                        14 => source ^ immediate,
                        15 => immediate * 0x1_0000,
                        _ => unreachable!(),
                    };
                    let opcode = (primary << 26) | (1 << 21) | (destination << 16) | immediate;
                    let handler = n64.rsp().instruction_tables.decode(opcode);
                    handler(&mut n64, opcode);
                    assert_eq!(
                        n64.rsp().registers.gpr[destination as usize],
                        expected,
                        "opcode={opcode:08x}"
                    );
                    assert_eq!(cpu_snapshot(&n64), snapshot);
                }
            }
        }
    }
}

#[test]
fn undocumented_vector_slots_preserve_flags_and_use_hardware_zero_or_noop_results() {
    let mut n64 = N64::default();
    start_rsp(&mut n64);
    let snapshot = cpu_snapshot(&n64);
    let left = [0, 1, 0x10, 0xffff, 0x7fff, 0x8000, 0xfffe, 0xffff];
    let right = [0, 2, 0x7fff, 0x7fff, 0, 0xffff, 0xfffe, 1];
    for function in [
        0x12, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1e, 0x1f, 0x2e, 0x2f, 0x37, 0x38, 0x39,
        0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f,
    ] {
        for (element, lanes) in ELEMENT_LANES.iter().enumerate() {
            for destination in [1, 2, 3] {
                n64.rsp_mut().registers.vpr[1] = pack(left);
                n64.rsp_mut().registers.vpr[2] = pack(right);
                n64.rsp_mut().registers.vpr[3] = u128::MAX;
                n64.rsp_mut().accumulator.accl = vector_from_bits(0x1234);
                n64.rsp_mut().accumulator.accm = vector_from_bits(0x5678);
                n64.rsp_mut().accumulator.acch = vector_from_bits(0x9abc);
                let flags = [0xaaaa, 0xbbbb, 0xcccc, 0xdddd, 0xeeee];
                n64.rsp_mut().vector_control.vcol = vector_from_bits(flags[0]);
                n64.rsp_mut().vector_control.vcoh = vector_from_bits(flags[1]);
                n64.rsp_mut().vector_control.vccl = vector_from_bits(flags[2]);
                n64.rsp_mut().vector_control.vcch = vector_from_bits(flags[3]);
                n64.rsp_mut().vector_control.vce = vector_from_bits(flags[4]);
                let before = n64.rsp().registers.vpr;
                let opcode = (18 << 26)
                    | ((16 + element as u32) << 21)
                    | (2 << 16)
                    | (1 << 11)
                    | (destination << 6)
                    | function;
                let handler = n64.rsp().instruction_tables.decode(opcode);
                handler(&mut n64, opcode);
                let noop = function == 0x37 || function == 0x3f;
                let mut expected = before;
                if !noop {
                    expected[destination as usize] = 0;
                }
                assert_eq!(n64.rsp().registers.vpr, expected, "opcode={opcode:08x}");
                let low = if noop {
                    0x1234
                } else {
                    pack(std::array::from_fn(|lane| {
                        left[lane].wrapping_add(right[lanes[lane]])
                    }))
                };
                assert_eq!(vector_to_bits(n64.rsp().accumulator.accl), low);
                assert_eq!(vector_to_bits(n64.rsp().accumulator.accm), 0x5678);
                assert_eq!(vector_to_bits(n64.rsp().accumulator.acch), 0x9abc);
                let actual = [
                    n64.rsp().vector_control.vcol,
                    n64.rsp().vector_control.vcoh,
                    n64.rsp().vector_control.vccl,
                    n64.rsp().vector_control.vcch,
                    n64.rsp().vector_control.vce,
                ]
                .map(vector_to_bits);
                assert_eq!(actual, flags);
                assert_eq!(cpu_snapshot(&n64), snapshot);
            }
        }
    }
}

#[test]
fn every_cop2_transfer_selector_and_element_matches_wire_bytes() {
    let mut n64 = N64::default();
    start_rsp(&mut n64);
    let snapshot = cpu_snapshot(&n64);
    for vector in 0..32usize {
        for element in 0..16usize {
            let bytes: [u8; 16] = std::array::from_fn(|index| (vector * 31 + index * 17) as u8);
            n64.rsp_mut().registers.vpr[vector] = u128::from_be_bytes(bytes);
            let opcode = (18 << 26) | (1 << 16) | ((vector as u32) << 11) | ((element as u32) << 7);
            let expected =
                i16::from_be_bytes([bytes[element], bytes[(element + 1) % 16]]) as i32 as u32;
            let handler = n64.rsp().instruction_tables.decode(opcode);
            handler(&mut n64, opcode);
            assert_eq!(n64.rsp().registers.gpr[1], expected);
            let value = 0x8765_4321;
            n64.rsp_mut().registers.gpr[1] = value;
            let opcode = opcode | (4 << 21);
            let handler = n64.rsp().instruction_tables.decode(opcode);
            handler(&mut n64, opcode);
            let mut expected = bytes;
            expected[element] = 0x43;
            if element < 15 {
                expected[element + 1] = 0x21;
            }
            assert_eq!(n64.rsp().registers.vpr[vector].to_be_bytes(), expected);
            assert_eq!(cpu_snapshot(&n64), snapshot);
        }
    }
    for selector in 0..32u32 {
        for value in WORD_EDGES {
            n64.rsp_mut().registers.gpr[1] = value;
            let opcode = (18 << 26) | (6 << 21) | (1 << 16) | (selector << 11);
            let handler = n64.rsp().instruction_tables.decode(opcode);
            handler(&mut n64, opcode);
            let opcode = (18 << 26) | (2 << 21) | (2 << 16) | (selector << 11);
            let handler = n64.rsp().instruction_tables.decode(opcode);
            handler(&mut n64, opcode);
            let expected = if selector % 4 < 2 {
                value as u16 as i16 as i32 as u32
            } else {
                value % 256
            };
            assert_eq!(
                n64.rsp().registers.gpr[2],
                expected,
                "selector={selector} value={value:08x}"
            );
            assert_eq!(cpu_snapshot(&n64), snapshot);
        }
    }
}

#[test]
fn all_vector_function_slots_are_callable_without_mutating_main_cpu() {
    let mut n64 = N64::default();
    start_rsp(&mut n64);
    let snapshot = cpu_snapshot(&n64);
    for function in 0..64 {
        for element in 0..16 {
            n64.rsp_mut().registers.vpr[1] =
                pack([0, 1, 0x8000, 0xffff, 0x7fff, 3, 0x1234, 0x5678]);
            n64.rsp_mut().registers.vpr[2] =
                pack([0xffff, 0x8000, 0, 1, 0x7fff, 0xfffe, 5, 0xabcd]);
            let opcode =
                (18 << 26) | ((16 + element) << 21) | (2 << 16) | (1 << 11) | (3 << 6) | function;
            let handler = n64.rsp().instruction_tables.decode(opcode);
            handler(&mut n64, opcode);
            assert_eq!(cpu_snapshot(&n64), snapshot, "opcode={opcode:08x}");
            assert!(n64.rsp().instruction.instruction_type == InstructionType::Vu);
        }
    }
}
