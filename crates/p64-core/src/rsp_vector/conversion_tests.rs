use super::*;
#[test]
fn numeric_lane_order_roundtrips() {
    for bits in [
        0,
        u128::MAX,
        0x000102030405060708090a0b0c0d0e0f,
        0x8000ffff7fff0001123456789abcdef0,
    ] {
        assert_eq!(vector_to_bits(vector_from_bits(bits)), bits);
    }

    {
        let a = vector_from_bits(0xffff7fff80000001ffff7fff80000001);
        let b = vector_from_bits(0x00010001000100010001000100010001);

        assert_eq!(
            vector_to_bits(add_wrapping(a, b)),
            0x00008000800100020000800080010002
        );
    }
}
