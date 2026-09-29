use crate::input;

mod types;
pub use types::{Config, Input, InputProfile};

impl Default for Config {
    fn default() -> Self {
        let input_profiles =
            std::collections::HashMap::from([("Default".to_owned(), input::get_default_profile())]);

        Self {
            input: Input {
                input_profile_binding: std::array::from_fn(|_| "Default".to_owned()),
                controller_assignment: Default::default(),
                input_profiles,
            },
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Config {
    pub fn load(directory: &std::path::Path) -> std::io::Result<Self> {
        let current = directory.join("input.json");
        match std::fs::read(&current) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                match std::fs::read(directory.join("config.json")) {
                    Ok(bytes) => {
                        let document: serde_json::Value = serde_json::from_slice(&bytes)?;
                        if document.get("input").is_some() {
                            Ok(serde_json::from_value(document)?)
                        } else {
                            Ok(Self::default())
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        Ok(Self::default())
                    }
                    Err(error) => Err(error),
                }
            }
            Err(error) => Err(error),
        }
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        crate::storage::atomic_write(path, &serde_json::to_vec_pretty(self)?)
    }
}
