use crate::rom::RomInfo;
use std::{collections::HashMap, path::PathBuf};

pub(crate) struct RomLibraryState {
    pub roms: Vec<PathBuf>,
    pub folder: Option<PathBuf>,
    pub info: HashMap<PathBuf, RomInfo>,
    pub query: String,
}
