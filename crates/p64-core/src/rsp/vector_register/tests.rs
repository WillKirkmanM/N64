use super::*;

const REGISTER: u128 = 0x0123_4567_89ab_cdef_fedc_ba98_7654_3210;

#[test]
fn byte_lanes_match_big_endian_wire_bytes_and_wrap() {
    let bytes = REGISTER.to_be_bytes();

    for element in u8::MIN..=u8::MAX {
        let offset = usize::from(element) % bytes.len();

        assert_eq!(get_vpr_byte(REGISTER, element), bytes[offset]);
        let mut expected = bytes;
        expected[offset] = 0xa5;
        let mut actual = REGISTER;
        modify_vpr_byte(&mut actual, 0xa5, element);

        assert_eq!(actual.to_be_bytes(), expected);
    }
}
