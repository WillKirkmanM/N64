use super::*;

pub(super) fn set_keys(default_keys: &mut [(bool, i32); 18], mappings: &[(usize, Scancode)]) {
    for &(index, scancode) in mappings {
        default_keys[index] = (true, scancode as i32);
    }
}

pub(super) fn set_controller_buttons_initially(
    default_buttons: &mut [(bool, i32); 14],
    mappings: &[(usize, Button)],
) {
    for &(index, button) in mappings {
        default_buttons[index] = (true, button as i32);
    }
}

pub(super) fn set_controller_axis_initially(
    default_axis: &mut [(bool, i32, i16); 18],
    mappings: &[(usize, Axis, i16)],
) {
    for &(index, axis, direction) in mappings {
        default_axis[index] = (true, axis as i32, direction);
    }
}

pub fn get_default_profile() -> crate::config::InputProfile {
    let mut default_keys = [(false, 0); 18];
    set_keys(
        &mut default_keys,
        &[
            (Input::RDPad.as_usize(), Scancode::D),
            (Input::LDPad.as_usize(), Scancode::A),
            (Input::DDPad.as_usize(), Scancode::S),
            (Input::UDPad.as_usize(), Scancode::W),
            (Input::StartButton.as_usize(), Scancode::Return),
            (Input::ZTrig.as_usize(), Scancode::Z),
            (Input::BButton.as_usize(), Scancode::LCtrl),
            (Input::AButton.as_usize(), Scancode::LShift),
            (Input::RCButton.as_usize(), Scancode::L),
            (Input::LCButton.as_usize(), Scancode::J),
            (Input::DCButton.as_usize(), Scancode::K),
            (Input::UCButton.as_usize(), Scancode::I),
            (Input::RTrig.as_usize(), Scancode::C),
            (Input::LTrig.as_usize(), Scancode::X),
            (Input::AxisLeft.as_usize(), Scancode::Left),
            (Input::AxisRight.as_usize(), Scancode::Right),
            (Input::AxisUp.as_usize(), Scancode::Up),
            (Input::AxisDown.as_usize(), Scancode::Down),
        ],
    );

    let mut default_controller_buttons = [(false, 0); 14];
    set_controller_buttons_initially(
        &mut default_controller_buttons,
        &[
            (Input::RDPad.as_usize(), Button::DPadRight),
            (Input::LDPad.as_usize(), Button::DPadLeft),
            (Input::DDPad.as_usize(), Button::DPadDown),
            (Input::UDPad.as_usize(), Button::DPadUp),
            (Input::StartButton.as_usize(), Button::Start),
            (Input::BButton.as_usize(), Button::X),
            (Input::AButton.as_usize(), Button::A),
            (Input::RTrig.as_usize(), Button::RightShoulder),
            (Input::LTrig.as_usize(), Button::LeftShoulder),
        ],
    );

    let mut default_controller_axis = [(false, 0, 0); 18];
    set_controller_axis_initially(
        &mut default_controller_axis,
        &[
            (Input::ZTrig.as_usize(), Axis::TriggerLeft, 1),
            (Input::RCButton.as_usize(), Axis::RightX, 1),
            (Input::LCButton.as_usize(), Axis::RightX, -1),
            (Input::DCButton.as_usize(), Axis::RightY, 1),
            (Input::UCButton.as_usize(), Axis::RightY, -1),
            (Input::AxisLeft.as_usize(), Axis::LeftX, -1),
            (Input::AxisRight.as_usize(), Axis::LeftX, 1),
            (Input::AxisUp.as_usize(), Axis::LeftY, -1),
            (Input::AxisDown.as_usize(), Axis::LeftY, 1),
        ],
    );

    crate::config::InputProfile {
        keys: default_keys,
        controller_buttons: default_controller_buttons,
        controller_axis: default_controller_axis,
        joystick_buttons: Default::default(),
        joystick_hat: Default::default(),
        joystick_axis: Default::default(),
    }
}
