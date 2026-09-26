use sdl2::{
    controller::{Axis, Button},
    joystick::HatState,
};

const BUTTONS: &[Button] = &[
    Button::A,
    Button::B,
    Button::X,
    Button::Y,
    Button::Back,
    Button::Guide,
    Button::Start,
    Button::LeftStick,
    Button::RightStick,
    Button::LeftShoulder,
    Button::RightShoulder,
    Button::DPadUp,
    Button::DPadDown,
    Button::DPadLeft,
    Button::DPadRight,
    Button::Misc1,
    Button::Paddle1,
    Button::Paddle2,
    Button::Paddle3,
    Button::Paddle4,
    Button::Touchpad,
];
const CARDINAL_HATS: &[HatState] = &[
    HatState::Centered,
    HatState::Up,
    HatState::Right,
    HatState::Down,
    HatState::Left,
];

const AXES: &[Axis] = &[
    Axis::LeftX,
    Axis::LeftY,
    Axis::RightX,
    Axis::RightY,
    Axis::TriggerLeft,
    Axis::TriggerRight,
];

pub(super) fn button_from_id(id: i32) -> Option<Button> {
    BUTTONS.iter().copied().find(|button| *button as i32 == id)
}

pub(super) fn axis_from_id(id: i32) -> Option<Axis> {
    AXES.iter().copied().find(|axis| *axis as i32 == id)
}

pub(super) fn stick_axis_from_id(id: i32) -> Option<Axis> {
    axis_from_id(id).filter(|axis| {
        matches!(
            axis,
            Axis::LeftX | Axis::LeftY | Axis::RightX | Axis::RightY
        )
    })
}

pub(super) fn cardinal_hat_from_id(id: i8) -> Option<HatState> {
    CARDINAL_HATS.iter().copied().find(|hat| *hat as i8 == id)
}
