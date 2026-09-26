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
