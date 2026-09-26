use super::*;

#[test]
fn valid_profile_ids_preserve_sdl_bindings() {
    for button in BUTTONS {
        assert_eq!(button_from_id(*button as i32), Some(*button));
    }

    for axis in AXES {
        assert_eq!(axis_from_id(*axis as i32), Some(*axis));
    }
}

#[test]
fn malformed_profile_ids_are_rejected() {
    for id in [-1, i32::MIN, i32::MAX, 21, 255] {
        assert_eq!(button_from_id(id), None);
    }

    for id in [-1, i32::MIN, i32::MAX, 6, 255] {
        assert_eq!(axis_from_id(id), None);
    }
}

#[test]
fn stick_and_hat_decoding_preserves_profile_restrictions() {
    for axis in AXES {
        assert_eq!(
            stick_axis_from_id(*axis as i32).is_some(),
            matches!(
                axis,
                Axis::LeftX | Axis::LeftY | Axis::RightX | Axis::RightY
            )
        );
    }

    for id in i8::MIN..=i8::MAX {
        let expected = match id {
            0 => Some(HatState::Centered),
            1 => Some(HatState::Up),
            2 => Some(HatState::Right),
            4 => Some(HatState::Down),
            8 => Some(HatState::Left),
            _ => None,
        };

        assert_eq!(cardinal_hat_from_id(id), expected);
    }
}
