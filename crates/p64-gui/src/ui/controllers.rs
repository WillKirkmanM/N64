use super::InputControllers;
use crate::config::Input;

impl InputControllers {
    pub(super) fn bind(&mut self, input: &Input) {
        let joysticks = self
            .joystick_subsystem
            .as_ref()
            .expect("SDL joysticks initialized");
        let controllers = self
            .controller_subsystem
            .as_ref()
            .expect("SDL controllers initialized");
        let joystick_count = joysticks.num_joysticks().unwrap_or(0);
        let mut taken = std::collections::HashSet::new();

        for (slot, assignment) in input.controller_assignment.iter().enumerate() {
            let Some(guid) = assignment else {
                continue;
            };

            let index = (0..joystick_count).find(|index| {
                !taken.contains(index)
                    && joysticks
                        .device_guid(*index)
                        .is_ok_and(|current| current.to_string() == *guid)
            });
            let Some(index) = index else {
                println!("Could not bind assigned controller");
                continue;
            };

            taken.insert(index);
            if input.input_profile_binding[slot] == "Default" {
                self.controllers[slot].controller = controllers.open(index).ok();
            }

            if self.controllers[slot].controller.is_none() {
                match joysticks.open(index) {
                    Ok(joystick) => self.controllers[slot].joystick = Some(joystick),
                    Err(_) => println!("could not connect joystick"),
                }
            }
        }
    }
}
