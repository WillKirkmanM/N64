use super::{Backend, Engine, USAGE};
use std::{ffi::OsString, path::PathBuf};

#[derive(Debug)]
pub(super) struct Options {
    pub(super) rom: PathBuf,
    pub(super) backend: Backend,
    pub(super) artifact: Option<PathBuf>,
    pub(super) stop_on_completion: bool,
    pub(super) unthrottled: bool,
}

impl Options {
    pub(super) fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut arguments = arguments.into_iter();
        let rom = arguments.next().ok_or(USAGE)?;
        let mut options = Self {
            rom: rom.into(),
            backend: Backend::Interpreter,
            artifact: None,
            stop_on_completion: false,
            unthrottled: false,
        };

        while let Some(argument) = arguments.next() {
            match argument.to_str() {
                Some("--backend") => {
                    let value = arguments.next().ok_or("Missing backend")?;
                    options.backend = match value.to_str() {
                        Some("interpreter") => Backend::Interpreter,
                        Some("jit") => Backend::Jit,
                        Some("aot") => Backend::Aot,
                        _ => return Err(format!("Unknown backend: {}", value.to_string_lossy())),
                    };
                }
                Some("--aot") => {
                    options.artifact = Some(arguments.next().ok_or("Missing artifact path")?.into())
                }
                Some("--stop-on-completion") => options.stop_on_completion = true,
                Some("--unthrottled") => options.unthrottled = true,
                _ => return Err(format!("Unknown option: {}", argument.to_string_lossy())),
            }
        }

        if options.backend == Backend::Aot && options.artifact.is_none() {
            return Err("AOT backend requires --aot FILE".into());
        }

        Ok(options)
    }

    pub(super) fn create_engine(&self) -> Result<Engine, String> {
        match self.backend {
            Backend::Interpreter => Ok(Engine::interpreter()),
            Backend::Jit => Ok(Engine::jit()),
            Backend::Aot => {
                let rom = p64_core::rom::try_read_rom(&self.rom)
                    .map_err(|error| format!("Read ROM: {error}"))?;
                let artifact = self
                    .artifact
                    .as_ref()
                    .ok_or("AOT backend requires --aot FILE")?;
                let metadata = std::fs::metadata(artifact)
                    .map_err(|error| format!("Read artifact metadata: {error}"))?;

                if metadata.len() > p64_core::aot::MAX_ARTIFACT_BYTES as u64 {
                    return Err("AOT artifact exceeds 128 MiB".into());
                }

                let bytes =
                    std::fs::read(artifact).map_err(|error| format!("Read artifact: {error}"))?;

                p64_core::aot::load_artifact(&bytes, &rom)
                    .map_err(|error| format!("Invalid AOT artifact: {error}"))
            }
        }
    }
}
