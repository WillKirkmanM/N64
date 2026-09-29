use crate::input::{BINDING_COUNT, BUTTON_COUNT, CONTROLLER_COUNT};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct InputProfile {
    pub keys: [(bool, i32); BINDING_COUNT],
    pub controller_buttons: [(bool, i32); BUTTON_COUNT],
    pub controller_axis: [(bool, i32, i16); BINDING_COUNT],
    pub joystick_buttons: [(bool, u32); BUTTON_COUNT],
    pub joystick_hat: [(bool, u32, i8); BUTTON_COUNT],
    pub joystick_axis: [(bool, u32, i16); BINDING_COUNT],
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct Input {
    pub input_profiles: std::collections::HashMap<String, InputProfile>,
    pub input_profile_binding: [String; CONTROLLER_COUNT],
    pub controller_assignment: [Option<String>; CONTROLLER_COUNT],
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub input: Input,
}
