use super::{
    cpu_snapshot,
    operands::{pack, ELEMENT_LANES},
};
use crate::{
    rsp::start_rsp,
    rsp_vector::{vector_from_bits, vector_to_bits},
    N64,
};

fn flags(bits: u8) -> u128 {
    pack(std::array::from_fn(|lane| {
        if bits & (1 << lane) == 0 {
            0
        } else {
            u16::MAX
        }
    }))
}

#[test]
fn vector_add_compare_logic_merge_and_accumulator_slots_match_signed_lane_oracles() {
    let mut n64 = N64::default();
    start_rsp(&mut n64);
    let cpu = cpu_snapshot(&n64);
    let left = [0u16, 1, 0xffff, 0x7fff, 0x8000, 0x8001, 0x5555, 0xaaaa];
    let right = [0u16, 0xffff, 1, 0x7fff, 0x8000, 0x7fff, 0xaaaa, 0x5555];
    for function in [
        0x10, 0x11, 0x13, 0x14, 0x15, 0x1d, 0x20, 0x21, 0x22, 0x23, 0x27, 0x28, 0x29, 0x2a, 0x2b,
        0x2c, 0x2d,
    ] {
        for (element, lanes) in ELEMENT_LANES.iter().enumerate() {
            for destination in [1, 2, 3] {
                for carry_low in [0u8, 0xff, 0x55, 0xaa] {
                    for carry_high in [0u8, 0xff, 0x55, 0xaa] {
                        n64.rsp_mut().registers.vpr[1] = pack(left);
                        n64.rsp_mut().registers.vpr[2] = pack(right);
                        n64.rsp_mut().registers.vpr[3] = u128::MAX;
                        let low_initial = pack([0x1111; 8]);
                        let middle_initial = pack([0x2222; 8]);
                        let high_initial = pack([0x3333; 8]);
                        n64.rsp_mut().accumulator.accl = vector_from_bits(low_initial);
                        n64.rsp_mut().accumulator.accm = vector_from_bits(middle_initial);
                        n64.rsp_mut().accumulator.acch = vector_from_bits(high_initial);
                        n64.rsp_mut().vector_control.vcol = vector_from_bits(flags(carry_low));
                        n64.rsp_mut().vector_control.vcoh = vector_from_bits(flags(carry_high));
                        n64.rsp_mut().vector_control.vccl = vector_from_bits(flags(0x5a));
                        n64.rsp_mut().vector_control.vcch = vector_from_bits(flags(0xa5));
                        n64.rsp_mut().vector_control.vce = vector_from_bits(flags(0x69));
                        let mut result = [0u16; 8];
                        let mut accumulator = [0u16; 8];
                        let mut new_low = carry_low;
                        let mut new_high = carry_high;
                        let mut compare_low = 0x5a;
                        let mut compare_high = 0xa5;
                        if matches!(function, 0x10 | 0x11 | 0x14 | 0x15 | 0x20..=0x23 | 0x27) {
                            new_low = 0;
                            new_high = 0;
                        }
                        if matches!(function, 0x20..=0x23) {
                            compare_low = 0;
                            compare_high = 0;
                        }
                        for lane in 0..8 {
                            let a = left[lane];
                            let b = right[lanes[lane]];
                            let signed_a = i32::from(a as i16);
                            let signed_b = i32::from(b as i16);
                            let carry = i32::from(carry_low & (1 << lane) != 0);
                            let high = carry_high & (1 << lane) != 0;
                            let output = match function {
                                0x10 => signed_a + signed_b + carry,
                                0x11 => signed_a - signed_b - carry,
                                0x13 => signed_a.signum() * signed_b,
                                0x14 => {
                                    if u32::from(a) + u32::from(b) > u32::from(u16::MAX) {
                                        new_low |= 1 << lane;
                                    }
                                    i32::from(a) + i32::from(b)
                                }
                                0x15 => {
                                    if a < b {
                                        new_low |= 1 << lane;
                                    }
                                    if a != b {
                                        new_high |= 1 << lane;
                                    }
                                    i32::from(a) - i32::from(b)
                                }
                                0x1d => match element {
                                    8 => 0x3333,
                                    9 => 0x2222,
                                    10 => 0x1111,
                                    _ => 0,
                                },
                                0x20..=0x23 | 0x27 => {
                                    let select = match function {
                                        0x20 => {
                                            signed_a < signed_b || (a == b && carry != 0 && high)
                                        }
                                        0x21 => a == b && !high,
                                        0x22 => a != b || high,
                                        0x23 => {
                                            signed_a > signed_b || (a == b && !(carry != 0 && high))
                                        }
                                        0x27 => 0x5a & (1 << lane) != 0,
                                        _ => unreachable!(),
                                    };
                                    if function != 0x27 && select {
                                        compare_low |= 1 << lane;
                                    }
                                    i32::from(if select { a } else { b })
                                }
                                0x28 => i32::from(a & b),
                                0x29 => i32::from(u16::MAX - (a & b)),
                                0x2a => i32::from(a | b),
                                0x2b => i32::from(u16::MAX - (a | b)),
                                0x2c => i32::from(a ^ b),
                                0x2d => i32::from(u16::MAX - (a ^ b)),
                                _ => unreachable!(),
                            };
                            accumulator[lane] = if function == 0x1d {
                                0x1111
                            } else {
                                output as u16
                            };
                            result[lane] = if matches!(function, 0x10 | 0x11 | 0x13) {
                                output.clamp(-32768, 32767) as u16
                            } else {
                                output as u16
                            };
                        }
                        let opcode = (18 << 26)
                            | ((16 + element as u32) << 21)
                            | (2 << 16)
                            | (1 << 11)
                            | (destination << 6)
                            | function;
                        let handler = n64.rsp().instruction_tables.decode(opcode);
                        handler(&mut n64, opcode);
                        assert_eq!(
                            n64.rsp().registers.vpr[destination as usize],
                            pack(result),
                            "opcode={opcode:08x} carry={carry_low:02x}/{carry_high:02x}"
                        );
                        assert_eq!(
                            vector_to_bits(n64.rsp().accumulator.accl),
                            pack(accumulator)
                        );
                        assert_eq!(vector_to_bits(n64.rsp().accumulator.accm), middle_initial);
                        assert_eq!(vector_to_bits(n64.rsp().accumulator.acch), high_initial);
                        assert_eq!(
                            vector_to_bits(n64.rsp().vector_control.vcol),
                            flags(new_low)
                        );
                        assert_eq!(
                            vector_to_bits(n64.rsp().vector_control.vcoh),
                            flags(new_high)
                        );
                        assert_eq!(
                            vector_to_bits(n64.rsp().vector_control.vccl),
                            flags(compare_low)
                        );
                        assert_eq!(
                            vector_to_bits(n64.rsp().vector_control.vcch),
                            flags(compare_high)
                        );
                        assert_eq!(vector_to_bits(n64.rsp().vector_control.vce), flags(0x69));
                        assert_eq!(cpu_snapshot(&n64), cpu);
                    }
                }
            }
        }
    }
}
