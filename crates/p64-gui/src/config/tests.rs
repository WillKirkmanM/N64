use super::*;

#[test]
fn default_config_keeps_first_run_controller_profile_usable() {
    for config in [Config::default(), Config::new()] {
        assert!(config.input.input_profiles.contains_key("Default"));
        assert!(config
            .input
            .input_profile_binding
            .iter()
            .all(|profile| profile == "Default"));
        assert!(config
            .input
            .controller_assignment
            .iter()
            .all(Option::is_none));
    }
}
