use std::path::Path;

const INTERNAL_NAME_BYTES: usize = 20;
const INTERNAL_NAME_OFFSET: usize = 32;

mod info;
pub use info::RomInfo;

impl RomInfo {
    pub fn matches(&self, query_lowercase: &str) -> bool {
        self.file_name.to_lowercase().contains(query_lowercase)
            || self.internal_name.to_lowercase().contains(query_lowercase)
    }

    pub fn new(path: &Path) -> Option<Self> {
        let file_name = path.file_stem()?.to_string_lossy().into_owned();

        let buffer = p64_core::rom::try_read_rom(path).ok()?;
        let internal_name = String::from_utf8_lossy(
            &buffer[INTERNAL_NAME_OFFSET..INTERNAL_NAME_OFFSET + INTERNAL_NAME_BYTES],
        )
        .trim_matches(|value: char| value == '\0' || value.is_whitespace())
        .to_owned();
        let md5 = format!("{:x}", md5::compute(&buffer));

        Some(RomInfo {
            file_name,
            internal_name,
            md5,
        })
    }
}
