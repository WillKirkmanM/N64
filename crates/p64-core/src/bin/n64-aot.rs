use p64_core::{
    aot::{artifact_record_count, compile_artifact},
    rom::try_read_rom,
};
use std::{env, fs, path::PathBuf};

const USAGE: &str = "Usage: n64-aot <ROM.z64|ROM.v64|ROM.n64|ROM.zip> <output.p64aot>";

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();

    if args.len() != 2 {
        return Err(USAGE.into());
    }

    let input = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[1]);

    if input == output {
        return Err("ROM input and artifact output must differ".into());
    }

    let rom = try_read_rom(&input).map_err(|error| format!("Read ROM: {error}"))?;
    let artifact = compile_artifact(&rom)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .map_err(|e| format!("Create artifact (output must not exist): {e}"))?;
    use std::io::Write;
    file.write_all(&artifact)
        .map_err(|e| format!("Write artifact: {e}"))?;
    let records = artifact_record_count(&artifact)?;
    println!(
        "Compiled {} ROM bytes into {} native scalar/block records ({} artifact bytes): {}",
        rom.len(),
        records,
        artifact.len(),
        output.display()
    );

    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
