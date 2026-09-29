use super::*;
use sdl2::keyboard::Scancode;

#[test]
fn repeated_keys_and_duplicate_axis_motion_do_not_advance_capture() {
    let mut capture = ProfileCapture::default();
    let key = |repeat| Event::KeyDown {
        timestamp: 0,
        window_id: 0,
        keycode: None,
        scancode: Some(Scancode::A),
        keymod: sdl2::keyboard::Mod::NOMOD,
        repeat,
    };

    assert!(matches!(
        capture.capture(Input::AButton, key(true)),
        CaptureResult::Ignored
    ));
    assert!(matches!(
        capture.capture(Input::AButton, key(false)),
        CaptureResult::Bound
    ));
    let axis = |value| Event::JoyAxisMotion {
        timestamp: 0,
        which: 0,
        axis_idx: 1,
        value,
    };

    assert!(matches!(
        capture.capture(Input::AxisUp, axis(24576)),
        CaptureResult::Ignored
    ));
    assert!(matches!(
        capture.capture(Input::AxisUp, axis(-30000)),
        CaptureResult::Bound
    ));
    assert!(matches!(
        capture.capture(Input::AxisDown, axis(-30000)),
        CaptureResult::Ignored
    ));
    assert!(matches!(
        capture.capture(Input::AxisDown, axis(30000)),
        CaptureResult::Bound
    ));
    let profile = capture.into_profile();

    assert_eq!(
        profile.keys[Input::AButton.as_usize()],
        (true, Scancode::A as i32)
    );
    assert_eq!(
        profile.joystick_axis[Input::AxisUp.as_usize()],
        (true, 1, -1)
    );
    assert_eq!(
        profile.joystick_axis[Input::AxisDown.as_usize()],
        (true, 1, 1)
    );
}

#[test]
fn stick_binding_ignores_button_and_hat_events() {
    let mut capture = ProfileCapture::default();
    for input in [
        Input::AxisUp,
        Input::AxisDown,
        Input::AxisLeft,
        Input::AxisRight,
    ] {
        assert!(matches!(
            capture.capture(
                input,
                Event::JoyButtonDown {
                    timestamp: 0,
                    which: 0,
                    button_idx: 1
                }
            ),
            CaptureResult::Ignored
        ));
        assert!(matches!(
            capture.capture(
                input,
                Event::JoyHatMotion {
                    timestamp: 0,
                    which: 0,
                    hat_idx: 0,
                    state: HatState::Up
                }
            ),
            CaptureResult::Ignored
        ));
    }
    assert!(capture
        .into_profile()
        .joystick_buttons
        .iter()
        .all(|binding| !binding.0));
}
