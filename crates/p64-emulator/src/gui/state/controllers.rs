use crate::{
    config::{ControllerSettings, UNASSIGNED_CONTROLLER},
    controllers::ConnectedController,
};
use p64_gui::input::CONTROLLER_COUNT;

pub(crate) struct ControllerState {
    pub assignments: [Option<String>; CONTROLLER_COUNT],
    pub configure_profile: bool,
    pub profile_name: String,
    pub available: Vec<ConnectedController>,
    pub selected_controller: [i32; CONTROLLER_COUNT],
    pub selected_profile: [String; CONTROLLER_COUNT],
    pub profiles: Vec<String>,
}

impl Default for ControllerState {
    fn default() -> Self {
        Self {
            assignments: Default::default(),
            configure_profile: false,
            profile_name: String::new(),
            available: Vec::new(),
            selected_controller: [UNASSIGNED_CONTROLLER; CONTROLLER_COUNT],
            selected_profile: std::array::from_fn(|_| "Default".to_owned()),
            profiles: vec!["Default".to_owned()],
        }
    }
}

impl ControllerState {
    pub fn resolve_connected(&mut self) {
        self.selected_controller = std::array::from_fn(|slot| {
            self.assignments[slot]
                .as_ref()
                .and_then(|guid| {
                    self.available
                        .iter()
                        .position(|device| &device.guid == guid)
                })
                .map(|index| index as i32)
                .unwrap_or(UNASSIGNED_CONTROLLER)
        });
    }

    pub fn select_controller(&mut self, slot: usize, index: i32) {
        let assignment = if index == UNASSIGNED_CONTROLLER {
            None
        } else {
            let Some(device) = usize::try_from(index)
                .ok()
                .and_then(|index| self.available.get(index))
            else {
                return;
            };
            Some(device.guid.clone())
        };
        let Some(binding) = self.assignments.get_mut(slot) else {
            return;
        };
        *binding = assignment;
        self.resolve_connected();
    }

    pub fn settings(&self) -> ControllerSettings {
        ControllerSettings {
            controller_guids: Some(self.assignments.clone()),
            selected_controller: self.selected_controller,
            selected_profile: self.selected_profile.clone(),
        }
    }
}
