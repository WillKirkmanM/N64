use super::*;
use std::{ffi::OsString, path::PathBuf};

fn parse(arguments: &[&str]) -> Result<Options, String> {
    Options::parse(arguments.iter().map(|value| OsString::from(*value)))
}

#[test]
fn default_and_aot_options_preserve_paths_and_host_pacing() {
    let default = parse(&["a ROM.z64"]).unwrap();

    assert_eq!(default.backend, Backend::Interpreter);
    assert_eq!(default.rom, PathBuf::from("a ROM.z64"));
    assert!(!default.unthrottled);
    let aot = parse(&[
        "ROM.z64",
        "--backend",
        "aot",
        "--aot",
        "code cache.p64aot",
        "--stop-on-completion",
        "--unthrottled",
    ])
    .unwrap();

    assert_eq!(aot.backend, Backend::Aot);
    assert_eq!(aot.artifact, Some(PathBuf::from("code cache.p64aot")));
    assert!(aot.stop_on_completion && aot.unthrottled);
}

#[test]
fn malformed_options_return_errors_before_starting_emulation() {
    for arguments in [
        &[][..],
        &["ROM.z64", "--backend"],
        &["ROM.z64", "--backend", "unknown"],
        &["ROM.z64", "--backend", "aot"],
        &["ROM.z64", "--aot"],
        &["ROM.z64", "--unknown"],
    ] {
        assert!(parse(arguments).is_err(), "arguments={arguments:?}");
    }
}
