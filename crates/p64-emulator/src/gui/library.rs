use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

const VALID_ROM_EXTENSIONS: [&str; 3] = ["z64", "n64", "v64"];

pub(super) fn is_rom(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            VALID_ROM_EXTENSIONS
                .iter()
                .any(|expected| extension.eq_ignore_ascii_case(expected))
        })
}

pub(super) fn discover_roms(folder: Option<&Path>) -> std::io::Result<Vec<PathBuf>> {
    let Some(folder) = folder else {
        return Ok(Vec::new());
    };
    let mut paths = Vec::new();
    for entry in read_dir(folder)? {
        let entry = entry?;
        if entry.file_type()?.is_file() && is_rom(&entry.path()) {
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}
