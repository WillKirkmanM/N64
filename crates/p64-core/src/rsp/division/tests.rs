use super::*;
use crate::rsp_vector::{from_bytes_reversed, vector_to_bits};

#[test]
fn estimates_preserve_signed_boundaries_and_normalization() {
    let mut rsp = Rsp::default();
    rsp.vector.initialize_division_tables();
    for (input, reciprocal, square_root) in [
        (0, 0x7fffffff, 0x7fffffff),
        (1, 0x7fffc000, 0x7fffc000),
        (-1, 0x80003fff, 0x80003fff),
        (2, 0x3fffe000, 0x5a824000),
        (-2, 0xc0001fff, 0xa57dbfff),
        (32767, 0x00010040, 0x00b53200),
        (-32768, 0xffff0000, 0xffff0000),
        (32768, 0x0000ffff, 0x00b50480),
        (65535, 0x00008020, 0x00802000),
        (i32::MIN, 0xfffffffe, 0xffff4acd),
        (i32::MAX, 0x00000001, 0x0000b532),
        (0x12345678, 0x00000007, 0x0001e039),
        (-0x12345678, 0xfffffff8, 0xfffe1fc6),
    ] {
        assert_eq!(estimate(&rsp, input, DivisionKind::Reciprocal), reciprocal);
        assert_eq!(estimate(&rsp, input, DivisionKind::SquareRoot), square_root);
    }
}

#[test]
fn paired_alias_keeps_high_latch_and_reads_source_before_destination() {
    let mut rsp = Rsp::default();
    rsp.vector.initialize_division_tables();
    rsp.vector.shuffle[0] =
        from_bytes_reversed([15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
    let opcode = (3 << 16) | (3 << 6); // Source/destination v3, lane zero.
    modify_vpr_element(&mut rsp.registers.vpr[3], 1, 0);
    rsp.division.divout = 2;
    latch_high(&mut rsp, opcode);

    assert_eq!(rsp.division.divin, 1);
    assert!(rsp.division.divdp);
    assert_eq!(get_vpr_element(rsp.registers.vpr[3], 0), 2);
    assert_eq!(get_vpr_element(vector_to_bits(rsp.accumulator.accl), 0), 1);

    divide(&mut rsp, opcode, DivisionKind::Reciprocal, true);

    assert!(!rsp.division.divdp);
    assert_eq!(rsp.division.divout, 0);
    assert_eq!(get_vpr_element(rsp.registers.vpr[3], 0), 0x7fff);
    assert_eq!(get_vpr_element(vector_to_bits(rsp.accumulator.accl), 0), 2);
}
