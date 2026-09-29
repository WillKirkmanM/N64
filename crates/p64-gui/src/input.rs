mod packet;
mod profile;
use packet::{encode_stick, press_button};

use sdl2::controller::GameController;
use sdl2::keyboard::{KeyboardState, Scancode};
use tracing::error;

use crate::ui;

mod types;
pub use types::{Controllers, Input};

pub const CONTROLLER_COUNT: usize = 4;
pub const BUTTON_COUNT: usize = 14;
pub const BINDING_COUNT: usize = 18;
pub const MAX_AXIS_VALUE: f64 = 85.0;
const STICK_RADIUS: f64 = 95.0;

fn directional_axis(position: i16, direction: i16, invert: bool) -> f64 {
    if position as isize * direction as isize <= 0 {
        return 0.0;
    }

    let value = position as f64 * MAX_AXIS_VALUE / i16::MAX as f64;

    if invert {
        -value
    } else {
        value
    }
}

fn bind_axis(x: f64, y: f64) -> (f64, f64) {
    let magnitude = x.hypot(y);

    if magnitude > STICK_RADIUS {
        let scale_factor = STICK_RADIUS / magnitude;

        (x * scale_factor, y * scale_factor)
    } else {
        (x, y)
    }
}

pub fn set_joystick_axis(
    profile: &crate::config::InputProfile,
    joystick: &sdl2::joystick::Joystick,
) -> (f64, f64) {
    fn get_axis_value(
        profile: &crate::config::InputProfile,
        joystick: &sdl2::joystick::Joystick,
        axis: usize,
        invert: bool,
    ) -> f64 {
        if profile.joystick_axis[axis].0 {
            let Ok(axis_position) = joystick.axis(profile.joystick_axis[axis].1) else {
                return 0.0;
            };

            return directional_axis(axis_position, profile.joystick_axis[axis].2, invert);
        }

        0.0
    }

    let x = get_axis_value(profile, joystick, Input::AxisLeft.as_usize(), false)
        + get_axis_value(profile, joystick, Input::AxisRight.as_usize(), false);
    let y = get_axis_value(profile, joystick, Input::AxisDown.as_usize(), true)
        + get_axis_value(profile, joystick, Input::AxisUp.as_usize(), true);

    (x, y)
}

pub fn set_controller_axis(
    profile: &crate::config::InputProfile,
    controller: &GameController,
) -> (f64, f64) {
    fn get_axis_value(
        profile: &crate::config::InputProfile,
        controller: &GameController,
        axis: usize,
        invert: bool,
    ) -> f64 {
        if profile.controller_axis[axis].0 {
            let Some(axis_enum) = profile::stick_axis_from_id(profile.controller_axis[axis].1)
            else {
                return 0.0;
            };

            let axis_position = controller.axis(axis_enum);

            return directional_axis(axis_position, profile.controller_axis[axis].2, invert);
        }

        0.0
    }

    let x = get_axis_value(profile, controller, Input::AxisLeft.as_usize(), false)
        + get_axis_value(profile, controller, Input::AxisRight.as_usize(), false);
    let y = get_axis_value(profile, controller, Input::AxisDown.as_usize(), true)
        + get_axis_value(profile, controller, Input::AxisUp.as_usize(), true);

    (x, y)
}

pub fn set_axis_from_keys(
    profile: &crate::config::InputProfile,
    keyboard_state: &KeyboardState,
) -> (f64, f64) {
    fn get_key_value(
        profile: &crate::config::InputProfile,
        keyboard_state: &KeyboardState,
        axis: usize,
        value: f64,
    ) -> f64 {
        if profile.keys[axis].0 {
            if let Some(scancode) = Scancode::from_i32(profile.keys[axis].1) {
                if keyboard_state.is_scancode_pressed(scancode) {
                    return value;
                }
            }
        }

        0.0
    }

    let x = get_key_value(
        profile,
        keyboard_state,
        Input::AxisLeft.as_usize(),
        -MAX_AXIS_VALUE,
    ) + get_key_value(
        profile,
        keyboard_state,
        Input::AxisRight.as_usize(),
        MAX_AXIS_VALUE,
    );
    let y = get_key_value(
        profile,
        keyboard_state,
        Input::AxisDown.as_usize(),
        -MAX_AXIS_VALUE,
    ) + get_key_value(
        profile,
        keyboard_state,
        Input::AxisUp.as_usize(),
        MAX_AXIS_VALUE,
    );

    (x, y)
}

pub fn set_joystick_buttons(
    profile: &crate::config::InputProfile,
    i: usize,
    joystick: &sdl2::joystick::Joystick,
    keys: &mut u32,
) {
    if profile.joystick_buttons[i].0 {
        if let Ok(button) = joystick.button(profile.joystick_buttons[i].1) {
            if button {
                press_button(keys, i);
            }
        }
    }

    if profile.joystick_hat[i].0 {
        if let Ok(hat_state) = joystick.hat(profile.joystick_hat[i].1) {
            let Some(expected_state) = profile::cardinal_hat_from_id(profile.joystick_hat[i].2)
            else {
                return;
            };

            if hat_state == expected_state {
                press_button(keys, i);
            }
        }
    }

    if profile.joystick_axis[i].0 {
        if let Ok(axis_position) = joystick.axis(profile.joystick_axis[i].1) {
            if axis_position as isize * profile.joystick_axis[i].2 as isize > 0
                && axis_position.saturating_abs() > i16::MAX / 2
            {
                press_button(keys, i);
            }
        }
    }
}

pub fn set_controller_buttons(
    profile: &crate::config::InputProfile,
    i: usize,
    controller: &GameController,
    keys: &mut u32,
) {
    let (enabled, button) = profile.controller_buttons[i];

    if let Some(button) = enabled.then(|| profile::button_from_id(button)).flatten() {
        *keys |= u32::from(controller.button(button)) << i;
    }

    let (enabled, axis, direction) = profile.controller_axis[i];

    if let Some(axis) = enabled.then(|| profile::axis_from_id(axis)).flatten() {
        let position = controller.axis(axis);

        if position as isize * direction as isize > 0 && position.saturating_abs() > i16::MAX / 2 {
            press_button(keys, i);
        }
    }
}

pub fn get(ui: &mut ui::Ui, channel: usize) -> u32 {
    let Ok(events) = ui.sdl_context.event_pump() else {
        return 0;
    };

    let keyboard_state = events.keyboard_state();

    let Some((profile_name, profile)) = ui.profile_for_channel(channel) else {
        return 0;
    };

    let mut keys = 0;
    let controller = &ui.input_controllers.controllers[channel].controller;
    let joystick = &ui.input_controllers.controllers[channel].joystick;

    for i in 0..BUTTON_COUNT {
        if profile_name != "Default" || channel == 0 {
            let profile_key = profile.keys[i];

            if profile_key.0 {
                if let Some(scancode) = Scancode::from_i32(profile_key.1) {
                    if keyboard_state.is_scancode_pressed(scancode) {
                        press_button(&mut keys, i);
                    }
                }
            }
        }

        if let Some(joystick) = joystick {
            set_joystick_buttons(profile, i, joystick, &mut keys);
        } else if let Some(controller) = controller {
            set_controller_buttons(profile, i, controller, &mut keys);
        }
    }

    let (mut x, mut y) = if profile_name != "Default" || channel == 0 {
        set_axis_from_keys(profile, &keyboard_state)
    } else {
        (0.0, 0.0)
    };

    if let Some(joystick) = joystick {
        (x, y) = set_joystick_axis(profile, joystick);
    } else if let Some(controller) = controller {
        (x, y) = set_controller_axis(profile, controller);
    }

    let (bound_x, bound_y) = bind_axis(x, y);

    keys |= encode_stick(bound_x, bound_y);

    keys
}

pub fn list_controllers(ui: &ui::Ui) {
    let joystick_subsystem = match ui.input_controllers.joystick_subsystem.as_ref() {
        Some(subsystem) => subsystem,
        None => {
            error!("Joystick subsystem is not available");
            return;
        }
    };

    let num_joysticks = match joystick_subsystem.num_joysticks() {
        Ok(num) => num,
        Err(e) => {
            error!("Failed to get number of joysticks: {}", e);
            return;
        }
    };

    if num_joysticks == 0 {
        println!("No controllers connected");
        return;
    }

    for i in 0..num_joysticks {
        match joystick_subsystem.name_for_index(i) {
            Ok(name) => println!("{}: {}", i, name),
            Err(e) => error!("Failed to get name for joystick {}: {}", i, e),
        }
    }
}

pub fn assign_controller(ui: &mut ui::Ui, controller: u32, port: usize) {
    let Some(joystick_subsystem) = ui.input_controllers.joystick_subsystem.as_ref() else {
        return;
    };
    let Ok(num_joysticks) = joystick_subsystem.num_joysticks() else {
        return;
    };
    if !(1..=CONTROLLER_COUNT).contains(&port) {
        return;
    }

    if controller < num_joysticks {
        match joystick_subsystem.device_guid(controller) {
            Ok(guid) => {
                ui.input_config_mut().controller_assignment[port - 1] = Some(guid.to_string())
            }
            Err(e) => error!(
                "Failed to get device GUID for controller {}: {}",
                controller, e
            ),
        }
    } else {
        error!("Invalid controller number: {}", controller);
    }
}

pub fn bind_input_profile(ui: &mut ui::Ui, profile: String, port: usize) {
    if !(1..=CONTROLLER_COUNT).contains(&port) {
        return;
    }
    if ui.input_config().input_profiles.contains_key(&profile) {
        ui.input_config_mut().input_profile_binding[port - 1] = profile;
    } else {
        error!("Invalid profile name: {}", profile);
    }
}

pub fn clear_bindings(ui: &mut ui::Ui) {
    let input = ui.input_config_mut();
    input.controller_assignment.fill(None);
    input.input_profile_binding = std::array::from_fn(|_| "Default".to_owned());
}

mod capture;
mod configure;
mod defaults;
pub use configure::{configure_profile, draw_text};
pub use defaults::get_default_profile;
