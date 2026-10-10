use super::{SaveData, SaveTypes};
use std::{fs::read, io, path::Path};

const SAVE_EXTENSIONS: [&str; 5] = ["eep", "sra", "fla", "mpk", "romsave"];

#[cfg(test)]
mod tests;

impl SaveData {
    pub fn set_paths(&mut self, directory: &Path, id: &str, hash: &str) {
        let paths = [
            &mut self.eeprom.path,
            &mut self.sram.path,
            &mut self.flash.path,
            &mut self.mempak.path,
            &mut self.romsave.path,
        ];

        for (path, extension) in paths.into_iter().zip(SAVE_EXTENSIONS) {
            *path = directory.join(format!("{id}-{hash}.{extension}"));
        }
    }

    pub fn prepare_game(&mut self, directory: &Path, id: &str, hash: &str) -> io::Result<()> {
        if id.is_empty()
            || hash.is_empty()
            || !hash
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Save storage requires a ROM identity",
            ));
        }
        let file_id = if id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            id
        } else {
            "UNK"
        };
        self.write_dirty()?;
        let mut prepared = Self::default();
        prepared.set_paths(directory, file_id, hash);
        prepared.load()?;
        *self = prepared;
        Ok(())
    }

    pub fn load(&mut self) -> io::Result<()> {
        let saves = [
            (&self.eeprom.path, &mut self.eeprom.data),
            (&self.sram.path, &mut self.sram.data),
            (&self.flash.path, &mut self.flash.data),
            (&self.mempak.path, &mut self.mempak.data),
        ];
        for (path, data) in saves {
            if let Some(bytes) = read_optional(path)? {
                *data = bytes;
            }
        }
        if let Some(bytes) = read_optional(&self.romsave.path)? {
            self.romsave.data = super::document::decode_romsave(&bytes)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        }
        Ok(())
    }

    pub fn write_dirty(&mut self) -> io::Result<()> {
        let saves = [
            (SaveTypes::Eeprom16k, self.eeprom.dirty),
            (SaveTypes::Sram, self.sram.dirty),
            (SaveTypes::Flash, self.flash.dirty),
            (SaveTypes::Mempak, self.mempak.dirty),
            (SaveTypes::Romsave, self.romsave.dirty),
        ];

        let mut result = Ok(());
        for (kind, dirty) in saves {
            if !dirty {
                continue;
            }
            match self.write(kind) {
                Ok(()) => *self.dirty_flag(kind) = false,
                Err(error) if result.is_ok() => result = Err(error),
                Err(_) => {}
            }
        }
        result
    }

    fn dirty_flag(&mut self, kind: SaveTypes) -> &mut bool {
        match kind {
            SaveTypes::Eeprom4k | SaveTypes::Eeprom16k => &mut self.eeprom.dirty,
            SaveTypes::Sram => &mut self.sram.dirty,
            SaveTypes::Flash => &mut self.flash.dirty,
            SaveTypes::Mempak => &mut self.mempak.dirty,
            SaveTypes::Romsave => &mut self.romsave.dirty,
        }
    }

    pub fn write(&self, kind: SaveTypes) -> io::Result<()> {
        let (path, data) = match kind {
            SaveTypes::Romsave => {
                let bytes = serde_json::to_vec(&(&self.romsave.data, self.romsave.dirty))?;
                return super::atomic_write(&self.romsave.path, &bytes);
            }
            SaveTypes::Eeprom4k | SaveTypes::Eeprom16k => (&self.eeprom.path, &self.eeprom.data),
            SaveTypes::Sram => (&self.sram.path, &self.sram.data),
            SaveTypes::Flash => (&self.flash.path, &self.flash.data),
            SaveTypes::Mempak => (&self.mempak.path, &self.mempak.data),
        };

        super::atomic_write(path, data)
    }
}

fn read_optional(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}
