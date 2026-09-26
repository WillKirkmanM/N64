use super::{encode_stick, press_button};

#[test]
fn signed_axes_occupy_the_two_high_packet_bytes() {
    for x in i8::MIN..=i8::MAX {
        for y in [i8::MIN, -1, 0, 1, i8::MAX] {
            assert_eq!(
                encode_stick(f64::from(x), f64::from(y)).to_le_bytes(),
                [0, 0, x as u8, y as u8]
            );
        }
    }
}
