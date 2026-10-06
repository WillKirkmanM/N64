use super::*;

fn packed_words(words: [u16; 8]) -> u128 {
    u128::from_be_bytes(core::array::from_fn(|byte| {
        words[byte / 2].to_be_bytes()[byte % 2]
    }))
}

fn wire_words(register: u128) -> [u16; 8] {
    let bytes = register.to_be_bytes();

    core::array::from_fn(|lane| u16::from_be_bytes([bytes[lane * 2], bytes[lane * 2 + 1]]))
}

#[test]
fn fractional_accumulate_matches_scalar_carries_and_source_destination_aliases() {
    let left = [i16::MIN, i16::MAX, -1, 1, -12345, 12345, 30000, -30000];
    let right = [i16::MIN, i16::MAX, 1, -1, 23456, -23456, -30000, 30000];
    let initial = [
        0_i64,
        (1_i64 << 47) - 1,
        -(1_i64 << 47),
        0xffff,
        0xffff_ffff,
        -1,
        0x1234_5678_9abc,
        -0x1234_5678_9abc,
    ];

    for unsigned in [false, true] {
        for (left_register, right_register, destination) in
            [(1, 2, 3), (1, 2, 1), (1, 2, 2), (1, 1, 1)]
        {
            let mut machine = N64::default();
            machine.rsp.vector.shuffle[0] =
                from_bytes_reversed([15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
            machine.rsp.registers.vpr[left_register] = packed_words(left.map(|value| value as u16));
            machine.rsp.registers.vpr[right_register] =
                packed_words(right.map(|value| value as u16));
            machine.rsp.accumulator.acch = vector_from_bits(packed_words(initial.map(|value| {
                let bytes = value.to_be_bytes();

                u16::from_be_bytes([bytes[2], bytes[3]])
            })));
            machine.rsp.accumulator.accm = vector_from_bits(packed_words(initial.map(|value| {
                let bytes = value.to_be_bytes();

                u16::from_be_bytes([bytes[4], bytes[5]])
            })));
            machine.rsp.accumulator.accl = vector_from_bits(packed_words(initial.map(|value| {
                let bytes = value.to_be_bytes();

                u16::from_be_bytes([bytes[6], bytes[7]])
            })));
            let left =
                wire_words(machine.rsp.registers.vpr[left_register]).map(|value| value as i16);
            let right =
                wire_words(machine.rsp.registers.vpr[right_register]).map(|value| value as i16);
            let expected_accumulator = core::array::from_fn::<_, 8, _>(|lane| {
                let sum = initial[lane] + i64::from(left[lane]) * i64::from(right[lane]) * 2;
                let wrapped = sum.rem_euclid(1_i64 << 48);

                if wrapped >= 1_i64 << 47 {
                    wrapped - (1_i64 << 48)
                } else {
                    wrapped
                }
            });
            let expected_result = expected_accumulator.map(|value| {
                let middle = value.div_euclid(1_i64 << 16);

                if unsigned {
                    if middle < 0 {
                        0
                    } else if middle > i64::from(i16::MAX) {
                        u16::MAX
                    } else {
                        middle as u16
                    }
                } else {
                    middle.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16 as u16
                }
            });
            let opcode = ((left_register as u32) << 11)
                | ((right_register as u32) << 16)
                | ((destination as u32) << 6);

            if unsigned {
                vmacu(&mut machine, opcode);
            } else {
                vmacf(&mut machine, opcode);
            }

            assert_eq!(
                wire_words(machine.rsp.registers.vpr[destination]),
                expected_result
            );
            let high = wire_words(vector_to_bits(machine.rsp.accumulator.acch));
            let middle = wire_words(vector_to_bits(machine.rsp.accumulator.accm));
            let low = wire_words(vector_to_bits(machine.rsp.accumulator.accl));
            let actual = core::array::from_fn::<_, 8, _>(|lane| {
                let high = high[lane].to_be_bytes();
                let middle = middle[lane].to_be_bytes();
                let low = low[lane].to_be_bytes();
                let sign = if high[0] >= 128 { u8::MAX } else { 0 };

                i64::from_be_bytes([
                    sign, sign, high[0], high[1], middle[0], middle[1], low[0], low[1],
                ])
            });

            assert_eq!(actual, expected_accumulator);
        }
    }
}
