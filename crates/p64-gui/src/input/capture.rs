use super::{Input, BINDING_COUNT, BUTTON_COUNT};
use crate::config::InputProfile;
use sdl2::{
    event::{Event, WindowEvent},
    joystick::HatState,
};

// Three quarters of the signed 16-bit SDL axis range.
const AXIS_CAPTURE_THRESHOLD: i16 = 24_576;

pub(super) enum CaptureResult {
    Ignored,
    Bound,
    Closed,
}

#[derive(Default)]
pub(super) struct ProfileCapture {
    keys: [(bool, i32); BINDING_COUNT],
    buttons: [(bool, u32); BUTTON_COUNT],
    hats: [(bool, u32, i8); BUTTON_COUNT],
    axes: [(bool, u32, i16); BINDING_COUNT],
    last_axis: (bool, u32, i16),
}

impl ProfileCapture {
    pub fn capture(&mut self, input: Input, event: Event) -> CaptureResult {
        let index = input.as_usize();

        match event {
            Event::Window {
                win_event: WindowEvent::Close,
                ..
            } => return CaptureResult::Closed,
            Event::KeyDown {
                scancode: Some(scancode),
                repeat: false,
                ..
            } => {
                self.keys[index] = (true, scancode as i32);
            }
            Event::JoyButtonDown { button_idx, .. } if index < BUTTON_COUNT => {
                self.buttons[index] = (true, button_idx as u32);
            }
            Event::JoyHatMotion { hat_idx, state, .. }
                if index < BUTTON_COUNT && state != HatState::Centered =>
            {
                self.hats[index] = (true, hat_idx as u32, state as i8);
            }
            Event::JoyAxisMotion {
                axis_idx, value, ..
            } if value.saturating_abs() > AXIS_CAPTURE_THRESHOLD => {
                let binding = (true, axis_idx as u32, value / value.saturating_abs());

                if binding == self.last_axis {
                    return CaptureResult::Ignored;
                }

                self.axes[index] = binding;
                self.last_axis = binding;
            }
            _ => return CaptureResult::Ignored,
        }

        CaptureResult::Bound
    }

    pub fn into_profile(self) -> InputProfile {
        InputProfile {
            keys: self.keys,
            controller_buttons: Default::default(),
            controller_axis: Default::default(),
            joystick_buttons: self.buttons,
            joystick_hat: self.hats,
            joystick_axis: self.axes,
        }
    }
}

#[cfg(test)]
mod tests;
