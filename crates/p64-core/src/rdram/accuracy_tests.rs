use super::*;
use crate::builder::memory::Memory;

#[test]
fn mi_repeat_respects_bus_lane_start_and_programmed_length() {
    let mut n64 = N64::new();
    let value = 0x9abcdef1u32;

    for width in [1, 2, 4] {
        for start in (0..=16usize).step_by(width) {
            for length in 1..=128usize {
                n64.rdram.mem.data[0..256].fill(0xff);
                crate::mips::write_regs(&mut n64, 0, 0x100 | (length - 1) as u32, u32::MAX);
                let shift = ((4 - width - (start & 3)) * 8) as u32;
                let mask = match width {
                    1 => 0xff << shift,
                    2 => 0xffff << shift,
                    _ => u32::MAX,
                };

                let bus_value = value.wrapping_shl(shift);
                write_mem(&mut n64, (start & !3) as u64, bus_value, mask);
                let end = start + length.saturating_sub(start & 7);
                let pattern = bus_value.to_be_bytes();

                for index in 0..256 {
                    let expected = if index >= start && index < end {
                        pattern[index & 3]
                    } else {
                        0xff
                    };

                    assert_eq!(
                        n64.rdram.mem[index ^ n64.byte_swap],
                        expected,
                        "width={width} start={start} length={length} index={index}"
                    );
                }

                assert!(!crate::mips::repeat_enabled(&n64));
            }
        }
    }
}

#[test]
fn mi_repeat_doubleword_wraps_inside_two_kibibytes() {
    let mut n64 = N64::new();
    n64.rdram.mem.data[0..4096].fill(0xff);
    crate::mips::write_regs(&mut n64, 0, 0x100 | 127, u32::MAX);
    write_mem_double_repeat(&mut n64, 2040, 0);

    for index in 0..4096 {
        let expected = if index < 120 || (2040..2048).contains(&index) {
            0
        } else {
            0xff
        };

        assert_eq!(n64.rdram.mem[index ^ n64.byte_swap], expected);
    }
}

#[test]
fn partial_rdram_stores_preserve_other_bytes() {
    let mut n64 = N64::new();
    write_mem(&mut n64, 4, 0x01234567, u32::MAX);
    write_mem(&mut n64, 4, 0x89abcdef, 0xff000000);

    assert_eq!(read_mem_fast(&mut n64, 4, AccessSize::Word), 0x89234567);
    write_mem(&mut n64, 4, 0x12345678, 0x0000ffff);

    assert_eq!(read_mem_fast(&mut n64, 4, AccessSize::Word), 0x89235678);
}
