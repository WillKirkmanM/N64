use sdl2::{controller::GameController, joystick::Joystick};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Input {
    RDPad,
    LDPad,
    DDPad,
    UDPad,
    StartButton,
    ZTrig,
    BButton,
    AButton,
    RCButton,
    LCButton,
    DCButton,
    UCButton,
    RTrig,
    LTrig,
    XAxis,
    YAxis,
    AxisLeft,
    AxisRight,
    AxisUp,
    AxisDown,
}

impl Input {
    pub fn as_usize(self) -> usize {
        match self {
            Input::RDPad => 0,
            Input::LDPad => 1,
            Input::DDPad => 2,
            Input::UDPad => 3,
            Input::StartButton => 4,
            Input::ZTrig => 5,
            Input::BButton => 6,
            Input::AButton => 7,
            Input::RCButton => 8,
            Input::LCButton => 9,
            Input::DCButton => 10,
            Input::UCButton => 11,
            Input::RTrig => 12,
            Input::LTrig => 13,
            Input::XAxis => 16,
            Input::YAxis => 24,
            Input::AxisLeft => 14,
            Input::AxisRight => 15,
            Input::AxisUp => 16,
            Input::AxisDown => 17,
        }
    }
}

#[derive(Default)]
pub struct Controllers {
    pub controller: Option<GameController>,
    pub joystick: Option<Joystick>,
}
