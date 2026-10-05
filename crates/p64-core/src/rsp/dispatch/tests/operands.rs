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
