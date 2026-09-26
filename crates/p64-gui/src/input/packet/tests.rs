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

#[test]
fn button_packet_keeps_axes_and_other_buttons() {
    let mut packet = encode_stick(-85.0, 85.0);
    press_button(&mut packet, 0);
    press_button(&mut packet, 13);

    assert_eq!(packet, 0x55ab_2001);
}
