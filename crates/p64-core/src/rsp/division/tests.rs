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
