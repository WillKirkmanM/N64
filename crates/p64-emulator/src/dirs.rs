use dirs::{cache_dir, config_dir};

pub fn ensure_directories() {
    for directory in [config_dir(), cache_dir()].into_iter().flatten() {
        if let Err(error) = std::fs::create_dir_all(directory.join("p64")) {
            tracing::error!("Unable to create application directory: {error}");
        }
    }
}
